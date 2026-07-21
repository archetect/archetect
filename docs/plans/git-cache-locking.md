# Git source cache — concurrency & locking

## Context

`archetect-git-cache` clones/fetches/checks-out git sources into a shared cache
(`~/.cache/archetect/<hash>/` for archetect; `~/.cache/prova/plugins/<url>/<label>/` for prova). Two
things now make concurrent access to one cache dir a live hazard rather than a theoretical one:

1. **The gRPC server (`archetect server`) builds a fresh `Archetect` per render request**
   (`archetect-core/src/server/core.rs`). The in-process fetch-dedup guard (`fetched_sources`) is
   *per-instance by design*, so every concurrent request starts empty, believes it is the first to
   see the URL, and takes the network-eligible `fetch` path — concurrent clone / `git fetch
   --force --tags` / `checkout_tree` into the **same** dir. Nothing serializes these tasks (the
   MCP server, by contrast, holds a session mutex and rejects a second concurrent render, so it is
   safe *intra-process*).
2. **prova now uses a stable cache** (not a per-pid temp dir) and shares `~/.cache/archetect` for
   archetypes, so two separate OS processes (`prova` + `archetect`, or two CI jobs) can hit one dir.

There are two distinct hazards:

- **Writer–writer (W-W):** concurrent clone/fetch/checkout of one dir. Corruption, half-fetched
  refs, and — worst — `clone()`'s git2→CLI fallback does `remove_dir_all` on the destination, which
  is actively destructive to a concurrent reader or writer.
- **Reader–writer (R-W):** archetect multiplexes many refs into **one** clone via detached-HEAD
  checkout. A render *reading* templates for ref A shares the exact working tree that a concurrent
  request *checking out* ref B mutates, so the reader sees a mix of A and B (or files mid-fetch). A
  write-only lock does **not** fix this, because the reader holds nothing during its read.

## Phase 1 — write lock (shipped, v3.2.1 / prova v0.3.1)

Implemented in `archetect-git-cache/src/lib.rs` (`with_cache_lock`, `keyed_mutex`,
`acquire_file_lock`). `fetch()` and `checkout()` run their whole clone/fetch/checkout critical
section under a per-`cache_path` **exclusive** lock with two layers:

- **In-process keyed mutex** — a process-global `HashMap<cache_path, Arc<Mutex<()>>>`. Serializes
  many `Archetect` instances / server requests / threads in one process (the case `flock` alone
  can't cover, since `flock` is process-wide).
- **Cross-process advisory file lock** — `fs4` `flock` on a sibling `<parent>/<name>.lock` file
  (sibling, not inside `.git/`, so it exists *before* the very first clone). Serializes separate OS
  processes.

Because both `fetch()` (first sighting → network-eligible) and `checkout()` (later sighting →
local-only re-checkout) mutate the working tree, both take the lock. This **fully closes W-W**:
concurrent callers can no longer clobber one dir mid-clone/fetch/checkout, and the destructive
`remove_dir_all` can never run under another operation.

Proven by `tests/concurrency.rs`: 8 threads race a cold cache; exactly one clones, the rest reuse,
all resolve the same commit, working tree intact. (Cross-process rides on the same `flock` and isn't
unit-tested — it needs separate OS processes.)

**What Phase 1 deliberately does not fix:** the R-W hazard. Under Phase 1, a render reading ref A
from a multiplexed dir can still be disturbed by a concurrent `checkout` of ref B into that dir,
because the reader (the render) never holds the lock — the crate returns after checkout and the
caller reads afterward. In practice this only bites when the **same repo is used at different refs
concurrently** through one shared cache (plausible in server mode; rare on a laptop).

## Phase 2 — content-addressed by commit (implemented)

The reader–writer fix can't be "hold a read lock across the render", because an archetype render is
**interactive**: the source is read from Initialize through the whole prompt loop. A user answering
prompts and going to lunch would hold that lock for an hour and freeze every other session that needs
to move the same repo. The lock can't span the render — so the render must read something that
**can't change under it**.

The unit of isolation is the resolved **commit**, not the ref (a ref moves; a commit never does). The
cache is content-addressed:

```text
<cache_root>/
  sources/<repo-hash>/       bare mirror per repo URL — objects + refs, the fetch target.
  trees/<repo-hash>/<oid>/    immutable working tree at ONE commit, materialized from the mirror.
    <oid>.lease              sessions hold a SHARED flock for their lifetime; the reaper needs EXCLUSIVE.
    <oid>.used               last-use stamp (mtime) for retention.
```

`resolve(url, gitref, cache_root, opts) -> ResolvedSource { tree_dir, oid, freshness, lease }` does it
all under a short per-repo **write lock**: ensure the mirror, run the freshness gate (TTL +
`ls-remote`, silent when unchanged), resolve `ref → oid`, materialize the tree if absent (git2
`checkout_tree` with a `target_dir`; `git archive | tar` fallback), and take a shared **lease**. The
caller renders from `tree_dir` holding the returned `Lease` — **no lock spans the render**, because
the tree is immutable. A branch that moves mid-session resolves to a new oid → a new tree; in-flight
sessions keep theirs.

Why this beats the earlier options: it closes R-W **structurally** (a reader of commit A and a writer
producing commit B touch different dirs), needs no lock across the render (so long-lived interactive
sessions can't deadlock), and makes the gRPC/MCP per-request `Archetect` correct for free (each pins
its own immutable tree — the per-instance `mark_source_fetched` dedup is gone). The write lock now
guards only fetch/resolve/materialize — the scope a lock should have.

**Lease** is two-layered: a shared `flock` (cross-process) plus a process-global refcount (in-process,
because `flock` self-conflict across fds is unreliable on some platforms — the case a concurrent
server needs). It only ever excludes the reaper.

**Reaper** (`prune(cache_root, retention)` / `archetect cache prune`): walk `trees/`; for each whose
last-use exceeds `retention` (default **90 days**), reap it only if no session holds it (in-process
refcount **and** a non-blocking exclusive `flock`). Crash-leftover `.tmp-*` dirs are always removed.
Runs per-repo under the write lock so it never races a resolve.

**Freshness** default dropped 7d → **1 day** (a re-check is cheap and silent when unchanged;
content-addressing means a moving branch just adds a tree, it doesn't disturb anyone). Configurable
under `updates:` (`interval`, `retention`).

**Disk** is bounded: one tree per distinct *rendered* commit — throttled by the freshness gate for
branches, one-and-done for tags/revs, objects shared once in the bare mirror, and swept by the reaper.

Proven by the crate's `content_addressing.rs` (a tree per commit; a long-lived session isolated from
and non-blocking to a branch move; the reaper reaps unused but skips leased), `two_gate.rs`
(freshness), and `concurrency.rs` (concurrent resolve into one cold cache). Both archetect (via
`Source`, which holds the lease for the render) and prova (plugins via the run, archetypes via
archetect-core's `Source`) share this one crate.
