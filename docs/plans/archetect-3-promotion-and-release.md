# Plan: Promote Archetect 3 to Primary + Unify the Release System

Status: proposed
Date: 2026-06-25

## Goal

Make Archetect 3 the primary, default version of Archetect, with a single
canonical repository, a tag-driven release pipeline that serves both the 3.x and
2.x lines, correct Homebrew tap behavior (`archetect` = v3, `archetect@2` = v2),
and a `render-action` that installs v3 by default.

## Locked decisions

These were chosen up front and drive the rest of the plan:

1. **Repo strategy — consolidate into `archetect/archetect`.** v3 becomes the
   `main` branch of the canonical repo; v2 moves to a `2.x` maintenance branch.
   The separate `archetect/archetect-3` repo is retired/archived.
2. **Release trigger — tag-driven.** Pushing a `vX.Y.Z` tag builds, releases, and
   updates Homebrew. Version is derived from the tag. One workflow serves both
   lines (3.x tags on `main`, 2.x tags on `2.x`).
3. **Homebrew — v2 stays installable.** `brew install archetect` → latest v3;
   `brew install archetect@2` → latest v2. A 2.x release must **not** overwrite
   the main `archetect` formula.
4. **render-action — new major tag defaulting to v3.** Cut `render-action@v2`
   that defaults to a 3.x version and fixes the broken artifact name; existing
   `@v1` consumers keep working against v2 until they opt in.

## Current state (verified)

### Repositories

| Repo | Local path | Role today |
|------|-----------|-----------|
| `archetect/archetect` | `~/personal/archetect/archetect` | Canonical, stable v2 (2.1.0). Default `main`. Tags `vX.Y.Z` up to `v2.1.0`. |
| `archetect/archetect-3` | `~/personal/archetect/archetect-3` | v3 dev (3.0.0). Branches: `main` (2.1.1, unreleased), `main-v3` (v3 tip `8562a18`), `feature/client-server`. Uses **jj**. |
| `archetect/homebrew-tap` | `~/personal/archetect/homebrew-tap` | Tap. `archetect.rb` + `archetect@X.Y.Z.rb`, regenerated from an archetype on `repository_dispatch`. Currently pinned to stale 2.0.5. |
| `archetect/render-action` | `~/personal/archetect/render-action` | Composite action. Downloads from `archetect/archetect` releases. Default `v2.0.0`. Broken artifact name. No tags. |

### Git topology (critical for consolidation)

- `git merge-base main main-v3` (in archetect-3 repo) = `d12de5e` = **Release 2.1.0**.
- `archetect/archetect` `main` tip = `d12de5e` (2.1.0).
- archetect-3 `main-v3` descends from `d12de5e` → **fast-forward** onto canonical `main`.
- archetect-3 `main` = `d12de5e` + `29d47e1` ("Release 2.1.1 [skip ci]"). This 2.1.1
  commit is **only** in the archetect-3 repo, **never tagged/released**, and **not**
  on `main-v3`.

**Implication:** No rebase is actually required. Consolidation is branch moves +
fast-forward pushes. The lone reconciliation item is the unreleased 2.1.1 commit.

### Release workflows (both repos, nearly identical)

- Trigger: `workflow_dispatch` with `level` (major/minor/patch).
- `cargo-workspaces version <level> ... --allow-branch main` bumps + commits
  `Release %v [skip ci]` and creates the tag.
- Build matrix (5): linux x86_64, linux arm64, macos arm64, macos x86_64,
  windows x86_64.
- Artifacts: `archetect-vX.Y.Z-<platform>-<arch>.tar.gz` (+ `.sha256`); windows
  `.zip` + Inno Setup `-installer.exe`.
- GitHub release created draft → finalized `--latest`.
- Homebrew: `ph-fritsche/action-dispatch@v1`, `repository_dispatch` type
  `update-formula`, to `${owner}/homebrew-tap`, auth `REPOSITORY_DISPATCH_TOKEN`.
  Payload carries version + 8 archive URLs/sha256. URLs derive from
  `github.repository` → consolidating into `archetect/archetect` makes URLs correct
  automatically.

### Homebrew tap mechanics

- `.github/workflows/update_formula.yaml`: on `repository_dispatch: [update-formula]`,
  renders the `archetype/` with the dispatch payload as answers (via
  `archetect-actions/archetect-render@v1`), prunes versioned formulas to the newest
  5, commits + pushes.
- **Always regenerates `archetect.rb`** from the payload → any dispatch (incl. a
  future v2 patch) overwrites the main formula. Must be gated for coexistence.

### render-action

- Composite. Installs via:
  `curl -LO .../releases/download/$version/archetect-$version-linux_x64.tar.gz`.
- **Bug:** current releases produce `archetect-$version-linux-x86_64.tar.gz`
  (`linux-x86_64`, not `linux_x64`) → already broken for any release newer than the
  old naming. Default `v2.0.0`.
- Renders with `archetect render --headless -A "$ANSWERS" <source> <args> <dest>`.

## Target end state

- `archetect/archetect`: `main` = 3.x, `2.x` = v2 maintenance. Default branch `main`.
  Tag-driven releases for both lines from one `release.yml`.
- `archetect/archetect-3`: archived (read-only), README pointing to canonical repo.
- `homebrew-tap`: `archetect` → latest v3; `archetect@2` → latest v2; versioned
  `archetect@X.Y.Z` retained (pruned per major line). v2 release never touches the
  main formula.
- `render-action`: `@v2` defaults to 3.x, correct artifact name, verified against v3
  CLI; `@v1` untouched.

---

## Phase 0 — Pre-flight verification

### Results (verified 2026-06-25)

1. ✅ **Fast-forward confirmed.** `origin/main` (`d12de5e`, Release 2.1.0) *is* an
   ancestor of v3 `main-v3` (`8562a18`). Consolidation is a clean FF — no rebase, no
   force-push.
2. ✅ **Unreleased 2.1.1 (`29d47e1`) is a dead version-bump only** — touches just 3
   `Cargo.toml` files (2.1.0 → 2.1.1), no code. Nothing to port to v3. It can serve as
   the `2.x` tip as-is (next v2 patch bumps to 2.1.2), or the version can be reset.
3. ✅ **render-action CLI is compatible with v3.** v3 `cli.rs` defines `--answer-file`
   /`-A` (a **file path**) and `--headless`. render-action writes `answers.json` and
   passes its path to `-A`, so the existing invocation works against v3. (Note: `-A` is
   answer-*file*, not inline JSON — already true in v2.)
4. ✅ **Homebrew auto-update works; `REPOSITORY_DISPATCH_TOKEN` is healthy (org-level).**
   Initial alarm (tap "stuck at 2.0.5") was a **stale local clone** — the *remote* tap
   formula is at **2.1.0** with correct URLs. The v2.1.0 release run (2025-12-04, 12m,
   success) dispatched `update-formula`, and the tap's `Update archetect to 2.1.0`
   workflow ran successfully the same day. The token isn't a *repo* secret on
   `archetect/archetect` (only `ACTIONS_RUNNER_DEBUG`, `CRATES_IO_TOKEN`) because it
   lives at **org level** (couldn't enumerate — HTTP 403, not org admin — but it
   demonstrably works). homebrew-tap needs no dispatch token itself; it pushes with the
   default `GITHUB_TOKEN` via `permissions: write-all`.
   - **Carry-over for consolidation:** the org-level `REPOSITORY_DISPATCH_TOKEN` already
     covers `archetect/archetect`, so when v3 releases move there, the brew dispatch
     keeps working with no secret changes. (One historical failed run each on
     2.0.7/dispatch was retried and succeeded — transient, not a config problem.)

### Original checklist

Do these before changing anything.

1. **Confirm fast-forward feasibility against the live remote** (local `main` may lag
   `origin/main`):
   - `cd ~/personal/archetect/archetect && git fetch origin`
   - Confirm `origin/main` is still an ancestor of archetect-3 `main-v3`
     (`git merge-base --is-ancestor origin/main <main-v3-sha>`).
2. **Decide the fate of unreleased 2.1.1** (`29d47e1`): either
   (a) include it on the new `2.x` branch (recommended — it's the v2 tip), and/or
   (b) cherry-pick its substantive changes onto `main`/v3 if not already present.
   Confirm whether `8562a18` (v3) already contains the 2.1.1 changes; if it's only a
   version-bump commit, no port needed.
3. **Verify v3 CLI surface used by render-action** still works:
   `archetect render --headless -A '{"k":"v"}' <source> <dest>` against a v3 build.
   If `-A`/`--headless` changed, the render-action fix must adapt.
4. **Inventory secrets** on `archetect/archetect`: ensure `REPOSITORY_DISPATCH_TOKEN`
   exists and has `repository_dispatch` write on `homebrew-tap` (it already powers v2
   releases; confirm not expired).
5. **Snapshot/backups:** ensure both repos are fully pushed to GitHub before branch
   surgery.

---

## Phase 1 — Repo consolidation into `archetect/archetect`

All git ops in the archetect-3 working copy use **jj** (repo is jj-managed); the
canonical archetect repo uses git. Do not push `main` or move bookmarks without
explicit go-ahead (per workflow rules) — this phase is the one push that needs sign-off.

1. **Add canonical remote** in the archetect-3 working copy (or operate from a fresh
   clone of `archetect/archetect`). Plan assumes pushing from the archetect-3 repo,
   which holds all needed commits.
2. **Create the v2 maintenance branch first** (do this before moving `main`):
   - Push archetect-3 `main` (tip `29d47e1`, the 2.1.1 line) to
     `archetect/archetect` as a new branch **`2.x`**.
   - This is a fast-forward over `origin/main` (2.1.0) — safe.
3. **Move canonical `main` to v3:**
   - Push archetect-3 `main-v3` (tip `8562a18`) to `archetect/archetect` `main`.
   - Fast-forward from 2.1.0 → no force-push, no history rewrite.
4. **Carry over the v3-only branches** as needed: `feature/client-server` →
   push to canonical (or drop if obsolete; CLAUDE.md notes the gRPC branch is stale).
5. **Set GitHub default branch** of `archetect/archetect` to `main` (now v3). It
   already is `main`; the content changes, the pointer doesn't.
6. **Branch protection / rules:** protect `main` and `2.x`; allow tag pushes
   `v*` (release trigger). Restrict who/what can push the `Release ... [skip ci]`
   bump commits.
7. **Workspace `Cargo.toml`**: canonical `main` now carries `version = "3.0.0"`;
   `2.x` carries `2.1.x`. Confirm member crates that exist only in v3 (e.g.
   `archetect-aml`, `archetect-mcp`) come along — they do, since `main-v3` is the source.
8. **Retire `archetect/archetect-3`:**
   - Update its README: "Merged into archetect/archetect; development continues there."
   - Archive the repo on GitHub (read-only) once the first v3 release ships from the
     canonical repo (not before — keep it as a fallback during cutover).
9. **Reconcile docs/CLAUDE.md** references that point at `archetect-3` repo URLs to the
   canonical repo.

**Outcome check:** `archetect/archetect` has `main` (v3) + `2.x` (v2), default `main`,
both fast-forwarded, no history rewritten.

---

## Phase 2 — Tag-driven release pipeline (serves 3.x and 2.x)

Redesign release into two workflows. Apply to `main` (v3); back-port the same to
`2.x` so v2 patches can still ship.

### 2a. `prepare-release.yml` (optional convenience, keeps the easy button)

- Trigger: `workflow_dispatch` with `level` (major/minor/patch).
- Runs on the branch it's dispatched from (`main` or `2.x`).
- `cargo-workspaces version <level> -y --allow-branch <current> -m "Release %v [skip ci]"`
  to bump + commit, then **push the tag `vX.Y.Z`**.
- Pushing the tag is the only thing that triggers the actual build (2b). This
  decouples version-bump from build and makes builds branch-agnostic.

### 2b. `release.yml` (the build/release/publish, tag-driven)

- Trigger: `on: push: tags: ['v*']`.
- Derive `VERSION=${GITHUB_REF_NAME}` (e.g. `v3.0.0`). **Sanity-gate**: assert the
  tag matches the workspace `Cargo.toml` version; fail loudly on mismatch.
- **Channel detection** from the tag's major:
  - `v3.*` → `channel=stable-latest` (the current top line).
  - `v2.*` → `channel=maintenance` (do not mark `--latest`, do not touch main brew
    formula).
  - Generalize: "latest" = highest major that has ever released. Simplest robust
    rule: a tag is "latest" if its major ≥ the max major among existing
    non-prerelease tags. Implement by querying tags, or hardcode `3` as the current
    latest major and revisit at the next major.
- Build matrix: unchanged (5 targets) + windows installer.
- GitHub release: create with artifacts; `--latest` only when `channel=stable-latest`.
- Homebrew dispatch: include the new `channel`/`update_main` fields (Phase 3).
- Remove the `cargo-workspaces` bump from this workflow — version now comes from the
  tag (the bump lives in 2a or is done manually before tagging).

### 2c. Manual path (no button)

Documented alternative to 2a: bump `Cargo.toml`, commit `Release X [skip ci]`,
`git tag vX.Y.Z && git push --tags`. 2b does the rest.

**Outcome check:** `git push` a `v3.0.0` tag on `main` → full build + GitHub release
(`--latest`) + brew main update. A `v2.1.2` tag on `2.x` → build + release (not
`--latest`) + brew `archetect@2` update only.

---

## Phase 3 — Homebrew tap: v2/v3 coexistence

Changes in `archetect/homebrew-tap`: the rendering archetype, the workflow, and the
release payload contract.

### 3a. Payload contract (set by `release.yml` in Phase 2)

Add fields to the `update-formula` dispatch payload:

- `channel`: `stable-latest` | `maintenance`
- `update_main`: `true` only for `stable-latest`
- `major`: e.g. `3` or `2` (for the `archetect@<major>` formula)

(Existing fields stay: `binary`, `version`, `homepage`, `description`, 8×
archive URLs + sha256.)

### 3b. Tap archetype (`archetype/contents/Formula/...`)

- Keep generating the **versioned** formula `archetect@{{ version }}.rb` (e.g.
  `archetect@3.0.0.rb`, `archetect@2.1.2.rb`).
- Add a **major-line** formula `archetect@{{ major }}.rb` (e.g. `archetect@3.rb`,
  `archetect@2.rb`) that always tracks the latest patch of that major. This gives
  users `brew install archetect@2`.
- Gate the **main** `archetect.rb`: only (re)render it when `update_main == true`.
  Implement via the archetype script (Rhai today) conditionally rendering that file,
  or by the workflow choosing which targets to render.
- Class names: existing `pascal_case` + `AT{version|replace(".", "_")}` pattern;
  extend for major-only (`ArchetectAT2`).

### 3c. `update_formula.yaml`

- Pass the richer answers through (already forwards full `client_payload` as JSON).
- **Prune per major line**, not globally: keep newest 5 of `archetect@3.*` and
  newest 5 of `archetect@2.*` independently, so a v3 release never prunes v2
  versioned formulas. Adjust the `ls | sort -V | tail -n +6` logic to group by major.
- Never delete `archetect@2.rb` / `archetect@3.rb` (major aliases) or `archetect.rb`.

### 3d. README

- Document: `brew install archetect` (v3), `brew install archetect@2` (v2 line),
  `brew install archetect@3` (explicit v3 latest).

**Outcome check:** Dispatch a simulated v3 payload → `archetect.rb`, `archetect@3.rb`,
`archetect@3.0.0.rb` updated. Dispatch a v2 payload → `archetect@2.rb`,
`archetect@2.1.2.rb` updated, **`archetect.rb` untouched**.

---

## Phase 4 — render-action adopts v3

In `archetect/render-action`:

1. **Fix the artifact name** to match current releases:
   `archetect-$version-linux-x86_64.tar.gz` (and the extracted dir name). Verify
   against an actual v3 release asset before tagging.
2. **Bump default `version`** to the first v3 release (e.g. `v3.0.0`). Keep it
   overridable.
3. **Verify the render invocation** against v3:
   `archetect render --headless -A "$ANSWERS" <source> <args> <dest>` (Phase 0 step 3).
   Adjust flags if v3 changed them.
4. **Tag `@v2`** for this version; create/maintain a moving `v2` major tag. Leave the
   existing `@v1` line pointing at the v2-era behavior so current consumers don't break.
5. Update README usage to `archetect/render-action@v2`.
6. Optional: parameterize platform/arch so the action isn't linux-x64-only.

**Outcome check:** A workflow using `render-action@v2` renders a sample archetype with
a v3 binary in CI.

---

## Phase 5 — Cutover sequence & verification

Order matters: prepare the consumers to handle v3 *before* the first v3 release flips
`archetect` to v3.

1. Land Phase 3 (tap) and Phase 4 (render-action) changes — they're backward-safe
   (no v3 release exists yet; `update_main` simply hasn't fired).
2. Execute Phase 1 (consolidation) with sign-off.
3. Land Phase 2 workflows on `main` and `2.x`.
4. **Dry-run the release** on a throwaway pre-release tag (e.g. `v3.0.0-rc.1`):
   - Confirm matrix builds, artifact names, checksums, GitHub release, and that the
     brew dispatch lands and renders the expected formulas.
   - Treat `-rc`/prerelease tags as **not** `--latest` and **not** `update_main`.
5. **Cut `v3.0.0`** from `main`. Verify:
   - GitHub release marked latest with all platform assets + sha256 + installer.
   - `brew update && brew install archetect` installs v3; `archetect --version` = 3.0.0.
   - `brew install archetect@2` installs the v2 line.
6. **Smoke-test a v2 maintenance release** (`v2.1.2` from `2.x`) to confirm it
   publishes, updates `archetect@2`, and leaves `archetect` on v3.
7. **Archive `archetect/archetect-3`** once v3.0.0 is confirmed live from the
   canonical repo.
8. Announce: README badges, docs, and any catalogs/archetypes that pin a version.

---

## Risks & open items

- ~~**Unreleased 2.1.1 commit**~~ — RESOLVED (Phase 0): version-bump only, nothing to
  port to v3.
- ~~**render-action CLI compatibility**~~ — RESOLVED (Phase 0): `-A`/`--answer-file` and
  `--headless` exist in v3; existing invocation works.
- ~~**Homebrew auto-update broken / dispatch token missing**~~ — RESOLVED (Phase 0):
  false alarm from a stale local clone. Remote tap is at 2.1.0; org-level
  `REPOSITORY_DISPATCH_TOKEN` works and already covers `archetect/archetect`.
- **`--latest` / `update_main` logic** must be robust so a late v2 patch never demotes
  the `archetect` formula. Hardcoding "latest major = 3" is simplest; document the
  revisit-at-next-major.
- **`requires.archetect` version gates** in existing archetypes: v3 must still satisfy
  `requires: archetect: "2.x"` for the ~80 production archetypes (backwards-compat is a
  project invariant). The tap's own archetype declares `requires: 2.0.0` — confirm v3
  honors it.
- **render-action CLI compatibility**: `-A`/`--headless` must behave identically in v3
  for `@v1`→`@v2` to be a clean default bump.
- **Secrets**: `REPOSITORY_DISPATCH_TOKEN` must be present + valid on the canonical
  repo; expiry would silently skip brew updates.
- **GitHub redirects**: archiving `archetect-3` is fine since nothing public should
  point at it once consolidated; double-check no external docs/CI reference its release
  URLs.
- **jj/git interop** during Phase 1: ensure bookmark→branch mapping pushes the intended
  SHAs; verify on GitHub before archiving anything.

## Appendix — branch/tag layout after consolidation

```
archetect/archetect
├── main          → 3.x  (default)   tags: v3.0.0, v3.0.1, ...
├── 2.x           → 2.x maintenance  tags: v2.1.2, ...
└── (feature/*)   → as needed

homebrew-tap/Formula
├── archetect.rb          → latest v3
├── archetect@3.rb        → latest v3 patch
├── archetect@2.rb        → latest v2 patch
├── archetect@3.0.0.rb    → pinned (newest 5 of major 3)
└── archetect@2.1.2.rb    → pinned (newest 5 of major 2)

render-action
├── @v1  → v2-era behavior (default v2.x)
└── @v2  → v3 default, fixed artifact name
```
