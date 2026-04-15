# git2 with transparent fallback to `git` CLI

## Problem

Archetect currently has a soft split:

- `git2` — config reads, `Repository::open`, notes/timestamps
- `git` CLI — clone, fetch, add, commit, remote, push

The CLI dependency is a real install-time friction (anyone on a fresh
macOS / Windows / minimal container hits "git not found" before they
can render a public archetype), while also carrying the weight of all
the things libgit2 is bad at (credential helpers, hooks, signing).

We'd like archetect to work **without the `git` binary** for the
common case — rendering a public archetype / catalog — while falling
back to `git` transparently when auth or hook-dependent behavior is
needed.

## Design

Operations fall into three buckets, each gets a different strategy.

### Bucket A — fetch-class (try git2, fall back on failure)

Operations that are remote→local, idempotent, and where `libgit2`'s
shortcoming (auth) is also detectable:

- `git clone <url> <path>`
- `git fetch`
- `git ls-remote` (for catalog version resolution)

Strategy: **try git2 first; on failure, clean up partial state and
shell out to `git`**.

```text
try git2::Repository::clone(url, dest)
    on success → done
    on failure →
        remove dest if it was partially created
        shell out to `git clone <url> <dest>`
```

- **Why this works:** clones are all-or-nothing from the caller's
  POV, failure is detectable (any `git2::Error` qualifies for
  fallback — we don't need to classify auth vs. network), and
  the output state after fallback is identical.
- **Cost of false-positive fallback** (git2 failed for a non-auth
  reason): one extra `git clone` subprocess. Negligible.
- **Gain on happy path:** no process spawn, typed errors, works
  in sandboxes with no `git` binary installed.

### Bucket B — local-only reads (always git2)

- Config read (`user.name`, `user.email`, etc.)
- Repository open / status
- Tag list, log, blob read
- Notes read/write (our pull-timestamp tracking)

Strategy: **git2 only**. No fallback needed — no auth involved,
already works.

Current code already does this; no change.

### Bucket C — local commits & push (stay on `git` CLI)

- `git init`
- `git add`
- `git commit`
- `git push`
- `git remote add`

Strategy: **`git` CLI only**. Do *not* try git2 first and fall back.

The reason this bucket is different: **git2 silently succeeds while
doing the wrong thing.**

- `git2::Repository::commit` does not run `pre-commit`, `commit-msg`,
  or `pre-push` hooks. An archetype that sets up a repo with hooks
  would find its initial commit bypasses them.
- `git2::Repository::commit` does not GPG / SSH-sign even when
  `commit.gpgsign=true`. User's signing preference is silently
  ignored.
- Push auth — covered anyway by the "fall back on failure" idea, but
  for commits the fallback never triggers because git2 succeeds.

Fallback doesn't help when the primary path is technically successful
but semantically wrong. So bucket C stays on CLI.

This is consistent with the user's mental model: **"archetect needs
`git` for the operations that would need `git` anyway"** — auth, hooks,
signing. Without those concerns, archetect needs nothing.

## Implementation outline

1. Introduce a thin `git_io` module (new file:
   `archetect-core/src/git_io.rs`) exposing:
   - `clone(url, dest) -> Result<()>`     — bucket A
   - `fetch(repo_path) -> Result<()>`     — bucket A
   - `ls_remote(url) -> Result<Vec<Ref>>` — bucket A
   - (bucket B continues to use `git2` directly — no wrapper needed)
   - (bucket C continues to use `Command::new("git")` directly)

2. The bucket-A functions implement the try-git2-then-CLI dance.
   They also handle partial-state cleanup (if git2 created a dir
   before failing, remove it before shelling out).

3. Update `source.rs` callers to route bucket-A operations through
   `git_io`. No change to the lua `git` module (which is bucket C).

4. Add a feature flag or CLI override (`--force-git-cli`) for
   debugging — easy escape hatch if git2's clone misbehaves for
   a specific URL / provider.

5. Update `check_common.rs`: `git` becomes **recommended** rather
   than **required**. Only warn if absent.

## What this does NOT solve

- **Commit hooks** on the generated project's initial commit: still
  run, because bucket C stays on CLI. ✓
- **Credential helpers / gh auth / SSH** for private clones: still
  work via the CLI fallback. ✓
- **GPG/SSH commit signing**: still works because commits go through
  CLI. ✓
- **libgit2 HTTPS proxy quirks** / enterprise TLS: if they bite on a
  specific public URL, the fallback catches it automatically. ✓

## Open questions

- **Shallow clones.** `git clone --depth 1` is the norm for fetching
  an archetype. libgit2 supports shallow clones as of libgit2 1.7
  (git2 0.18+). Need to verify the Rust `git2` crate exposes the
  option cleanly.
- **Progress reporting.** The CLI prints `Cloning into...` etc.
  which we currently log to the user. git2 has `RemoteCallbacks`
  for equivalent progress — decide whether to match the output or
  simplify to a single "Fetching..." line.
- **Pull timestamp via git notes.** Already bucket B (git2) — no
  change.

## Non-goals

- Removing the `git` binary dependency entirely. It remains required
  for auth, commit hooks, and signing. This plan makes `archetect`
  **work without `git`** for the common public-catalog case, not
  eliminate the dependency.
- Replacing our Lua `git` module. That's authoring-side, where
  hooks/signing/credentials all matter.

## Scope estimate

- `git_io` module + three functions + tests: 1 day.
- Source-side rewiring: ~half a day (careful with cache dir
  semantics).
- CI update (Linux/macOS/Windows): verify public clones work
  without `git` installed in the image.

Total: ~2 days of focused work, low risk.
