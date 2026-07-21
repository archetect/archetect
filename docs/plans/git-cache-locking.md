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

## Phase 2 — reader–writer (design; not yet implemented)

Three options, roughly by invasiveness. The choice is a deliberate follow-up.

### (a) Shared-lock guard held across the render — *recommended*
`fetch()` / `checkout()` return a `CacheGuard` that holds a **shared** lock (readers coexist); a
`checkout` of a different ref needs the **exclusive** lock and therefore waits for readers to drain.
The caller (archetect-core's render flow, prova-archetect's render) holds the guard across
`archetype.render()`.
- **Pros:** fully closes R-W; keeps the current one-dir-many-refs layout and disk footprint; the RW
  file lock (`flock` `LOCK_SH`/`LOCK_EX`) already supports this cross-process.
- **Cons:** an API change (return a guard) touching the two render call sites; serializes a
  writer-for-ref-B against any in-flight reader of the same dir (correct, but reduces concurrency
  for the same repo). Needs a real in-process `RwLock` per dir (owned guard — likely `parking_lot`
  or a small self-managed registry) to pair with the cross-process `flock`.

### (b) Per-`(url,ref)` checkout dirs
Make archetect key the cache dir on `(url, ref)` (as prova's plugin cache already does), so each ref
gets its own dir and is checked out once.
- **Pros:** R-W disappears *structurally* — a reader of ref A and a writer of ref B touch different
  dirs; readers of an already-populated immutable (tag/rev) dir need no lock at all. Unifies both
  tools' layouts.
- **Cons:** changes archetect's cache-key derivation (`SourceType::create`); more disk (a full clone
  per ref) unless shared via a git object store / `--reference` / worktrees; a migration for
  existing `~/.cache/archetect/<hash>/` dirs. The Phase 1 write lock is still needed for the initial
  clone of each `(url,ref)` dir and for mutable-branch re-fetch.

### (c) Snapshot-under-lock
Under the Phase 1 write lock: fetch + checkout, then copy the checked-out tree to a private
per-render dir; release the lock; render from the private copy.
- **Pros:** no API change to hold a lock across the render; no layout change; readers never touch the
  shared tree beyond the locked window.
- **Cons:** a tree copy per render (cheap for small archetypes, wasteful for large ones); doesn't
  help a caller that legitimately wants to read the cache dir directly.

**Recommendation:** (a) — it closes R-W with the smallest conceptual change and no new disk cost,
and the cross-process RW `flock` is a natural extension of Phase 1. Revisit (b) if the per-`(url,ref)`
layout becomes desirable for other reasons (it would also let archetect hold two refs of one repo
live at once, which the multiplexed layout cannot).
