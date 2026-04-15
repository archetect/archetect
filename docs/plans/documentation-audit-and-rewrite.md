# Archetect 3: Documentation Audit and Rewrite

## Context

The documentation site at `/Users/jimmie/tmp/generated/archetect.github.io`
(publishing target `archetect.github.io`) was largely "vibe-coded" against a
mental model of the v3 scripting engine rather than the actual source. An
initial audit of two sections — `docs/cli/**` and `docs/scripting/**`
(plus `docs/reference/lua-api.mdx`) — turned up systemic factual errors:
hallucinated enum variants, wrong module namespacing, fictional features, and
invalid code examples that will not run.

This plan captures the verified inaccuracies, calls out the systemic patterns
to fix globally before per-section rewrites, lists the documentation sections
still pending audit, and proposes a remediation sequence.

Ground truth for this plan is the source tree at
`/Users/jimmie/personal/archetect/archetect-3` and the example archetypes at
`/Users/jimmie/personal/archetect/foundational-scratch`. The documentation
repo is a standalone Docusaurus project — all fixes happen there, not in the
source tree.

---

## Systemic Inaccuracies (Cross-Cutting)

These appear in multiple files and must be fixed with a single pass rather
than per-file, otherwise corrections will leak back in.

### 1. Rhai compatibility is fiction in v3

- **Claim (docs):** `docs/scripting/rhai-compat.mdx` states Rhai is retained
  as a frozen compatibility layer and that v2 archetypes render unchanged.
- **Source reality:** `archetect-core/src/script/mod.rs` contains only
  `pub mod lua;`. `archetect-core/src/archetype/archetype.rs:83-90` hard-errors
  on any `.rhai` file with the message "Rhai scripts (.rhai) are not supported…
  use archetect2 to render legacy archetypes". There is zero Rhai runtime.
- **Action:** Delete `docs/scripting/rhai-compat.mdx` or replace it with a
  single-paragraph "Rhai is not supported in v3; use `archetect2` for legacy
  archetypes" stub. Remove any sidebar link to it. Audit every other page for
  "Rhai" / "rhai" mentions and scrub those that imply live support.

### 2. Destination is never a bare positional

- **Claim (docs):** Examples across `render.mdx`, `catalog.mdx`, `global.mdx`,
  `connect.mdx`, `server.mdx` all show
  `archetect render <source> <dest>` or `archetect rust/cli ./my-cli`.
- **Source reality:** `archetect-bin/src/cli.rs:26-31` defines `render` with a
  single positional `source`; destination is the named option
  `--destination` / `--dest` (default `.`). The top-level `action` arg is a
  single positional with no sibling positional. `connect` has only `endpoint`
  plus render_args.
- **Action:** Rewrite every example that uses a bare second positional to
  either use `--destination <path>` explicitly or drop it and rely on the `.`
  default. Do a single grep pass for `archetect\s+\S+\s+\./` across all .mdx
  files.

### 3. `Existing` enum has wrong variants and wrong default

- **Claim (docs):** `Existing.Overwrite`, `Existing.Skip`, `Existing.Error`
  with `Overwrite` as default.
- **Source reality (UPDATED):** Source now registers
  `Existing.Overwrite`, `Existing.Preserve`, `Existing.Prompt`,
  `Existing.Error`. Default remains `Preserve`. The `Error` variant is
  new (per source-side #2 below) and hard-fails the render on conflict.
- **Action:** Replace `Existing.Skip` → `Existing.Preserve` everywhere.
  `Existing.Error` is now valid — KEEP doc references, but verify they
  describe the actual semantic (hard-fail, useful for CI / idempotent
  renders). Update the default callout to `Preserve`. Affected files:
  `docs/scripting/rendering.mdx:42,89-92`,
  `docs/scripting/index.mdx:54`,
  `docs/reference/lua-api.mdx:21,48,113`.

### 4. `archetect.switches` — REVERSED by source #6

- **Claim (docs):** `archetect.switches:contains(name)`.
- **Source reality (UPDATED):** `archetect.switches` now exists. Per
  source-side #6, both `switches` and `env` were consolidated under the
  `archetect` namespace (top-level globals removed entirely — no
  aliases). API is `archetect.switches.is_enabled(name)` (dot, not
  colon — it's a function on a sub-table, not a method).
- **Action:** Replace `archetect.switches:contains(X)` →
  `archetect.switches.is_enabled(X)` (note dot vs colon). Replace any
  bare `switches.is_enabled(X)` references with `archetect.switches.is_enabled(X)`.
  Same for any `env.os` / `env.arch` etc. → `archetect.env.os`. The
  flat-global forms are GONE — they no longer work. Affected:
  `docs/scripting/composition.mdx:111-113`,
  `docs/reference/lua-api.mdx:129`, plus a sweep for any plain `switches.`
  or `env.` calls in narrative pages.

### 5. Filesystem paths are XDG, not `~/.archetect/`

- **Claim (docs):** References to `~/.archetect/`, `~/.cache/archetect/`
  treated inconsistently; `ide.mdx` specifically says annotations install to
  `~/.archetect/lua/annotations/`.
- **Source reality:** v3 uses XDG via `XdgSystemLayout`:
  - config: `~/.config/archetect/`
  - cache: `~/.cache/archetect/`
  - data: `~/.local/share/archetect/`
  Lua IDE annotations live at `data_dir/lua/annotations`
  (`ide_subcommand.rs:18`). v2 still uses `~/.archetect/` but v2 is a
  different binary.
- **Action:** Grep for `~/.archetect` across all docs; replace with the
  correct XDG dir per context. Add a single "Filesystem Layout" reference
  page (may use `docs/reference/filesystem-layout.mdx` — already present;
  audit it against `archetect-core/src/system.rs`).

### 6. Method name: `prompt_multiselect` — REVERSED by source #3

- **Claim (docs):** `context:prompt_multiselect(...)`.
- **Source reality (UPDATED):** Per source-side #3, the method is now
  registered as `prompt_multiselect` (canonical). The old underscored
  name `prompt_multi_select` is kept as a deprecated alias that emits
  a LogWarn at runtime.
- **Action:** Docs were already correct on this one. NO replacement
  needed. Optionally add a one-liner note in `docs/scripting/prompting.mdx`
  or the lua-api reference saying `prompt_multi_select` is the
  deprecated form (will be removed in a future release).

### 7. Prompt methods return nothing — REVERSED by source #1

- **Claim (docs):** "The result is also returned, so you can use it inline."
- **Source reality (UPDATED):** Per source-side #1, all `prompt_*`
  methods now return the user-supplied value (string / integer / boolean
  / string[] depending on prompt type). `nil` is returned when an
  optional prompt is skipped. The context side-effect is unchanged.
- **Action:** Docs were already correct on this one. NO removal needed.
  Add a callout that the **return value is the user's raw input**, not
  case-expanded — case variants (set via `opts.cases`) are stored as
  context-side-effect keys; use `ctx:get("project-name")` to read them.

---

## Per-Section Confirmed Inaccuracies

### `docs/cli/`

| Doc location | Problem | Source |
|---|---|---|
| `render.mdx:10,26,41,47,56` | Bare `<dest>` positional (see systemic #2) | `cli.rs:26-31` |
| `catalog.mdx:22,24` | Bare `./my-cli` positional (see systemic #2) | `cli.rs:64-71` |
| `global.mdx:28` | Bare `./my-svc` positional (see systemic #2) | `cli.rs:41-47` |
| `connect.mdx:26-29`, `server.mdx:34` | `connect` shown with source positional; it has none, only `endpoint` | `cli.rs:196-205` |
| `ls.mdx:32` | Claims `-o`/`--offline` works; `ls` has no flags (no `render_args`) | `cli.rs:72-76` |
| `ls.mdx:19-29` | Tree output shown; actual is flat with `📂`/`📦` icons | `actions_subcommand.rs:31-44` |
| `index.mdx:18` | Lists `catalog` as a subcommand; it isn't — it's the `action` positional with default `"default"` | `cli.rs:64-71`, `main.rs:175-179` |
| `index.mdx:54` | `--config-file <path>`; actual value-name is `<config>` | `cli.rs:96` |
| `ide.mdx:13` | Annotations path hardcoded to `~/.archetect/lua/annotations/` (see systemic #5) | `ide_subcommand.rs:18` |
| `ide.mdx:19` | "containing `archetype.yaml`"; actually checks `MANIFEST_FILE_NAMES` list | `ide_subcommand.rs:39` |
| `ide.mdx` | Omits: `.luarc.json` is NOT overwritten if it already exists | `ide_subcommand.rs:49-53` |
| `cache.mdx:44` | "Equivalent to `rm -rf ~/.cache/archetect/`" — only contents removed; confirm prompt not documented | `cache_subcommand.rs:105-126` |
| `config.mdx:35` | "Creates the file if it doesn't exist" — only written on editor save | `config_subcommand.rs:19-43` |

Unverified from this pass (needs `archetect-core/src/check.rs` read):
- `check.mdx:22-23` claim that exit code is 0 even with warnings
- `check.mdx` specific checks (git presence, utilities, network)

### `docs/scripting/` and `docs/reference/lua-api.mdx`

| Doc location | Problem | Source |
|---|---|---|
| `rhai-compat.mdx` (entire file) | See systemic #1 | `script/mod.rs`, `archetype.rs:83-90` |
| `index.mdx:54` | `Existing.Skip/Error` (see systemic #3) | `modules.rs:908-910` |
| `index.mdx:54` | `archetect` described as holding switches+env; it holds only version fields + `answers()` | `modules.rs:187-199` |
| `index.mdx:47-57` | Missing globals: `Case`, `output`, `runtime`, `env`, `switches`, `format`, `exit`, `archetype` | `modules.rs` |
| `context.mdx:65-68` | `tostring(context)` shown as debug aid — `Context` has no `__tostring`; returns pointer-ish garbage | `context.rs` (no `add_meta_methods`) |
| `prompting.mdx:19-21` | Inline return value (see systemic #7) | `context.rs` |
| `prompting.mdx:59` | `prompt_multiselect` (see systemic #6) | `context.rs:608` |
| `prompting.mdx:73-80` | `optional` claimed text/editor-only; actually honored by `prompt_int`, `prompt_confirm`, `prompt_select`, `prompt_multi_select`, `prompt_list` | `context.rs:489,547,572,618,680` |
| `prompting.mdx` (options table) | Missing options: `cases`, `allow_other`, `other_label`, `answer_key` | `context.rs:199-206,575-578` |
| `prompting.mdx` (prompt list) | Missing type entirely: `prompt_list` | `context.rs:668` |
| `prompting.mdx:88` | "Validation is wired through `archetect-validations`" — not reached from Lua prompts | (no call sites) |
| `rendering.mdx:42,89-92` | `Existing.Skip/Error` + wrong default (see systemic #3) | `modules.rs:908-925` |
| `rendering.mdx:58-61` | `read_file(...)` — not a registered global; use `io.open`/`io.read` | (no registration) |
| `rendering.mdx:81` | Path template uses `context.var`; templates receive a flat context table — use bare `var` | (matches all real examples) |
| `casing.mdx:143-144` | "ATL does not ship case filters" — false, full filter suite is registered | `modules.rs:633-653` |
| `casing.mdx:55-59` | `Cases.all()` implies it includes Plural/Singular; actually 13 styles only | `cases.rs:75-91` |
| `git.mdx:29` | `branch` default documented as `"main"`; source omits `-b` and defers to git config | `require_modules.rs:421-428` |
| `git.mdx:35` | `repo:add({list})` — takes single `String`, not a table | `require_modules.rs:359` |
| `git.mdx:38` | `repo:status()` — method does not exist | `require_modules.rs:358-387` |
| `git.mdx:64` | Claims `GITHUB_TOKEN` required; `gh auth token` also accepted | `require_modules.rs:458-481` |
| `composition.mdx:111-113` | `archetect.switches:contains(...)` (see systemic #4) | `modules.rs:478-480` |
| `modules-and-helpers.mdx:16-18` | Missing require modules: `archetect.shell`, `archetect.archive`, `archetect.model`, `archetect.model.interactive` | `require_modules.rs:52-111` |
| `modules-and-helpers.mdx:25` | Missing globals (see `index.mdx:47-57` row above) | `modules.rs` |
| `modules-and-helpers.mdx:72-75` | `io.lines("manifest.txt")` shown as helper — resolves against process cwd, not archetype root; misleading | stdlib semantics |
| `logging.mdx` (whole) | Omits `output.print(msg)` and `output.banner(msg)`; "Always go through `log`" is overstated | `modules.rs:883-899` |
| `lua-api.mdx:21,48,113` | `Existing.Skip/Error` (see systemic #3) | `modules.rs:908-910` |
| `lua-api.mdx:36` | `prompt_multiselect` (see systemic #6) | `context.rs:608` |
| `lua-api.mdx:35-36` | Missing Context methods: `has(key)`, `contains(key, value)`, `prompt_list(...)` | `context.rs:357,360,668` |
| `lua-api.mdx:127-130` | `archetect.switches` (see systemic #4); `archetect.answers()` not listed | `modules.rs` |
| `lua-api.mdx:136` | `repo:add(list)` wrong + `status()` does not exist | `require_modules.rs:358-387` |
| `lua-api.mdx:133-138` | Missing require modules (see `modules-and-helpers.mdx:16-18`) | `require_modules.rs:52-111` |

---

## Sections Still Pending Audit

These were not exercised in the initial pass. Given the scripting section was
almost 100% wrong at the API level, treat them as untrusted until verified.

- `docs/templating/**` — verify ATL syntax claims, filter list, control-flow
  directives, path templating, partials/includes against
  `archetect-templating/**` (vendored MiniJinja fork 0.30.6) and the filters
  registered in `archetect-core/src/script/lua/modules.rs:633-653`.
- `docs/authoring/**` — especially `manifest.mdx` (verify against actual
  manifest deserialization in `archetect-core/src/archetype/manifest.rs` or
  equivalent), `components.mdx`, `libraries.mdx`, `catalogs.mdx`,
  `inflections.mdx`, `validations.mdx`, `regeneration.mdx`,
  `answers-and-switches.mdx`, `archetype-layout.mdx`.
- `docs/modeling/**` — verify against `archetect-aml` crate and
  `archetect.model` / `archetect.model.interactive` require modules.
- `docs/mcp/**` — four files beyond `index.mdx`; spot-check
  `mcp__claude_ai_Atlassian`-style tool names against
  `archetect-mcp/**` (server.rs tools already partially verified).
- `docs/patterns/**` — nine files, each presents a worked pattern. Every
  code block should be run through the same "does the API exist?" filter.
- `docs/reference/**` — every file other than `lua-api.mdx`:
  `aml-schema.mdx`, `answer-files.mdx`, `archetype-manifest.mdx`,
  `atl-grammar.mdx`, `cli-flags.mdx`, `configuration.mdx`, `errors.mdx`,
  `filesystem-layout.mdx`, `io-protocol.mdx`, `mcp-protocol.mdx`.
- `docs/getting-started/**` — six files. Installation, quick-start,
  your-first-archetype, workflows, concepts.
- `docs/intro.mdx` — top-of-site marketing copy; low risk but verify version
  claims and the "what it does" framing.

---

## Proposed Remediation Sequence

The scripting section is the most visible surface for authors and has the
highest defect rate, so it goes first. CLI is second because its errors
directly break users' first commands. Sections remain unaudited until
verified — do not "fix" them speculatively.

### Phase 1 — Global sweeps (before any per-page rewrites)

1. Grep and replace the systemic errors in a single coordinated pass:
   - `Existing.Skip` → `Existing.Preserve`
   - `Existing.Error` → `Existing.Prompt`
   - `archetect.switches:contains(X)` → `switches.is_enabled(X)`
   - `prompt_multiselect` → `prompt_multi_select`
   - `~/.archetect/` → XDG path per context
   - Bare second positional in CLI examples → `--destination <path>`
2. Delete or stub `docs/scripting/rhai-compat.mdx`; remove its sidebar link.
3. Remove inline-return claims from all `prompt_*` examples.

### Phase 2 — Rebuild the Lua API reference from source

4. Before touching narrative pages, regenerate `docs/reference/lua-api.mdx`
   directly from the source by walking
   `archetect-core/src/script/lua/{modules.rs,context.rs,require_modules.rs,cases.rs}`.
   This page is the canonical reference the narrative pages should link into.
   It must list:
   - Every global: `Context`, `Case`, `Cases`, `Existing`, `directory`,
     `template`, `catalog`, `log`, `output`, `archetect`, `archetype`,
     `runtime`, `env`, `switches`, `format`, `exit`
   - Every `Context` method with exact signature, arg table keys, return
     type (most return `()`)
   - Every require module (`archetect.shell`, `archetect.git`,
     `archetect.github`, `archetect.archive`, `archetect.model`,
     `archetect.model.interactive`) with its exported functions
   - Every registered ATL filter (case filters plus the inflection filters)

### Phase 3 — Narrative scripting pages

5. Rewrite `docs/scripting/{index,context,prompting,rendering,casing,git,
   logging,composition,modules-and-helpers}.mdx` to conform to the regenerated
   reference. Every code block must be pasted into a scratch archetype and
   executed end-to-end against the current `cargo build` of archetect-3
   before merge.

### Phase 4 — CLI pages

6. Rewrite `docs/cli/**` against `archetect-bin/src/cli.rs` and the
   `subcommands/` dispatchers. Verify each flag's short form, long form,
   value-name, default, and env var name (e.g. `ARCHETECT_ALLOW_EXEC`).
7. Remove the `catalog` entry from `docs/cli/index.mdx`'s subcommand table
   or reframe it as "the default action positional".

### Phase 5 — Audit remaining sections

8. Run the same two-agent parallel audit pattern over the pending sections
   (templating/authoring/modeling/mcp/patterns/reference/getting-started),
   producing a findings list that extends this document.
9. Apply Phase 1-style global sweeps for any new systemic errors discovered
   in those audits before per-page rewrites.

### Phase 6 — Regression prevention

10. Add a CI-time doc check to the docs repo that:
    - Extracts Lua code blocks from `.mdx` files and runs them through a
      minimal `lua` syntax check (at minimum `luac -p`).
    - Extracts CLI examples and dry-runs them with `archetect <cmd> --help`
      or a `--check-args-only` mode (add to CLI if not present).
    - Fails CI on "obvious fiction" indicators:
      `Existing\.(Skip|Error)`, `archetect\.switches`, `prompt_multiselect`,
      `~/\.archetect/` (unless in a v2 migration context).
11. Add a "Last verified against archetect-3 commit `<sha>`" footer to each
    reference page so reviewers can tell when the page has drifted.

---

## Proposed Source-Side Changes (archetect-3)

Several "documentation inaccuracies" above actually reflect places where the
*source* is the awkward side and the (hallucinated) documentation described a
more ergonomic API. Rather than mechanically forcing the docs to match the
source, each item below should be evaluated: does the docs' implicit design
win, or does the source? The ones where the docs' design wins should be
backported into the source before the docs are rewritten — otherwise we'll
ship awkward semantics to match awkward docs.

Items are tagged **[adopt docs-side design]** (change source to match the
intuitive doc), **[keep source, fix docs]** (source is correct, docs should
be rewritten), or **[needs decision]**.

**Status as of 2026-04-15** — most decisions are now made and shipped on
the `main-v3` branch (commits omsmp → uzolp). Each item below is annotated
with **[✅ shipped]**, **[⏭️ skipped]**, or **[⏳ doc-only]**. The doc
rewrite should reflect the *post-change* surface; do not document the
pre-change behavior described in some items.

### Lua API ergonomics

1. **Prompt methods should return the prompted value.** **[✅ shipped — commit ntqnk]**
   - All seven `prompt_*` methods now return the user-supplied value
     (`string?`, `integer?`, `boolean?`, `string?`, `string[]?`,
     `string[]?`, `string?` for text/int/confirm/select/multiselect/
     list/editor). `nil` is returned when an optional prompt is skipped.
   - Context side-effect unchanged: value is still stored under the key,
     and cases (when supplied) still expand into sibling keys. Use
     `ctx:get(key)` to read case-expanded variants — return value is
     always the user's raw input.
   - **Doc impact:** show inline-return examples freely. Add a callout
     about the return-value-is-raw semantic so authors understand
     `local n = ctx:prompt_text(...)` gives the typed value, not the
     pretty-cased one.

2. **`Existing.Error` variant added (no `Skip` alias).** **[✅ shipped — commit lqzru]**
   - Source now registers `Existing.Overwrite | Preserve | Prompt | Error`.
     Default still `Preserve`. `Error` hard-fails the render on conflict
     (returns a render error rather than overwriting / preserving /
     prompting).
   - `Skip` alias intentionally NOT added — one name per semantic.
     Use `Preserve`.
   - Wire format: `EXISTING_FILE_POLICY_ERROR = 4` in the proto enum.
   - **Doc impact:** four variants now valid. Show `Error` as the right
     choice for CI / idempotent renders ("every render should be a fresh
     destination; collisions mean the invocation was misconfigured").

3. **`prompt_multi_select` → `prompt_multiselect` (drop the underscore).** **[✅ shipped — commit ntqnk]**
   - Canonical: `prompt_multiselect`. Old name kept as a deprecated
     alias that emits a LogWarn at runtime.
   - **Doc impact:** use `prompt_multiselect` everywhere. Optionally
     mention the deprecated alias for migration purposes.

4. **`repo:add(list)` accepted; `repo:status()` skipped.** **[✅ shipped (partial) — commit uzolp]**
   - `repo:add` now accepts `string | string[]`. Both
     `repo:add('src/main.rs')` and
     `repo:add({ 'Cargo.toml', 'src/', '.github/workflows/' })` work.
     Empty tables and unknown types error out with a clear message.
   - `repo:status()` was DROPPED from scope — no clear use case in
     archetype-generation flows ("after initial commit, what's left is
     usually nothing by design"). If a need arises later, revisit.
   - **Doc impact:** document `repo:add(string | string[])`. Do NOT
     document `repo:status()` — it does not exist.

5. **`__tostring` on Context emits valid answer-file YAML.** **[✅ shipped — commit ysopq]**
   - `tostring(ctx)` now returns the context's data as YAML (single
     source of truth with `format.to_yaml(ctx)`). The output round-trips
     as an archetect answer file — `log.debug(tostring(ctx))` doubles as
     "what answers would reproduce this state".
   - **Doc impact:** show `tostring(ctx)`, `format.to_yaml(ctx)`,
     `format.to_json(ctx)` as three equivalent flavors. Mention the
     answer-file round-trip as a useful debugging trick.

6. **Consolidate `switches` / `env` under `archetect` — aliases REMOVED.** **[✅ shipped — commit zsxsn]**
   - `archetect.switches.is_enabled(name)` and `archetect.env.{os,arch,
     family,is_unix,is_windows,is_macos}` are the canonical surface.
   - Top-level `switches` and `env` globals were REMOVED entirely
     (we're in the v3 refinement phase; one name per thing).
   - **Doc impact:** any reference to bare `switches.is_enabled(...)` or
     `env.os` is now wrong — they error at runtime. Always use the
     `archetect.*` form. Sweep narrative pages for both.

7. **`file.exists` / `file.read` / `file.render` shipped.** **[✅ shipped — commit rqrtq]**
   - New top-level `file` module, single-file counterpart to
     `directory.*`. Functions:
     - `file.exists(path, opts?)` → boolean
     - `file.read(path, opts?)` → string
     - `file.render(source, ctx, opts?)` → ()
   - Default scope = archetype source root. Pass `{ scope = "cwd" }` on
     `exists` / `read` to resolve against the invocation working dir
     (rare — `.archetect.yaml` auto-detection and `-A` answer files
     already cover most per-invocation context). `file.render` has NO
     scope param — source always resolves against the archetype root.
   - Sandbox (always on, both scopes): no absolute paths, no `..`
     escapes, no `~` expansion.
   - Pairs with `format.from_yaml` / `from_json` / `from_toml` and
     `context:merge(table)` for the "if a defaults file exists, load it
     into context" pattern.
   - **Doc impact:** new `docs/scripting/files.mdx` (or merge into
     rendering.mdx). Document the three functions, both scopes, the
     sandbox, and the canonical "load defaults" pattern.

8. **`prompt_list` documentation.** **[⏳ doc-only — annotation already present]**
   - Method exists and is annotated in
     `archetect-core/lua/annotations/archetect.lua`. No source change.
   - **Doc impact:** narrative mention in `docs/scripting/prompting.mdx`.

### CLI ergonomics

9. **Destination as optional second positional.** **[✅ shipped — commit osvnn]**
    - Added `[destination]` as an optional positional on `render`,
      `global`, and the top-level action form. Resolution order:
      positional → `--destination`/`--dest` flag → `.`.
    - `archetect render <source> <dest>` now works (matches `git clone`,
      `cargo new`, v2 ergonomics).
    - Top-level form also takes `<dest>`:
      `archetect new-entity ./src/domain` for project-local actions
      (per the user note: an action can be a render, a catalog browse,
      or anything custom — the dest is for the render case).
    - **Doc impact:** examples can finally use the natural shape. Show
      both `archetect render <src> <dest>` and `archetect <action> <dest>`.

10. **`ls` is already offline-only.** **[✅ doc-only update — commit uzolp]**
    - Source already walks only in-memory catalog config (never fetches);
      the docs' claim about `--offline` was confused. Sharpened the
      `ls` long_about to make the offline-only behavior explicit. NO
      flag added — there's nothing to opt into.
    - **Doc impact:** drop the `--offline` reference. State that `ls`
      lists the in-memory catalog tree from `.archetect.yaml` (project
      or global), shallow — leaf entries pointing at remote catalogs
      are listed but not recursed into.

11. **`catalog` subcommand — SKIPPED.** **[⏭️ skipped]**
    - No clear use case beyond "force the manifest catalog to render
      skipping the .lua script". Speculative; revisit if a real need
      surfaces.
    - **Doc impact:** remove any reference to `archetect catalog` as
      a subcommand. The first positional (action) handles catalog
      navigation: `archetect <path>` (no `catalog` keyword needed).

12. **`git.init` defaults to `-b main`.** **[✅ shipped — commit uzolp]**
    - `git.init` now passes `-b main` by default. Override via
      `{ branch = "..." }`. Deterministic across machines regardless
      of `init.defaultBranch` config.
    - **Doc impact:** state the default explicitly. Show
      `git.init(nil, { branch = "develop" })` as the override pattern.

13. **`cache clear`: per-entry removal with confirmation.** **[⏳ doc-only]**
    - Source unchanged. Per-entry removal is safer than the docs' claimed
      `rm -rf`-equivalent.
    - **Doc impact:** correct the description in
      `docs/cli/cache.mdx:44`.

14. **`config edit`: file written only on editor save.** **[⏳ doc-only]**
    - Source unchanged. The "creates the file if it doesn't exist"
      claim is misleading — the file appears when the editor saves it,
      not before.
    - **Doc impact:** correct in `docs/cli/config.mdx:35`.

15. **`ide` setup .luarc.json is hash-idempotent.** **[✅ shipped — commit uzolp]**
    - No `--force` flag. Instead, the setup compares on-disk content
      hash with what we'd write. Identical → silent no-op. Differs (or
      missing) → overwrite + log "Created" or "Updated".
    - Avoids staleness when the annotations dir moves (e.g., XDG path
      change) without requiring user intervention.
    - **Doc impact:** describe the idempotent behavior. Authors can
      re-run `archetect ide setup` after every binary upgrade safely.

16. **`connect` / `server` doc fixes.** **[⏳ doc-only]**
    - Subcommands are correct as-is; the docs invented positional args.
    - **Doc impact:** remove fictional source/path positionals from
      `docs/cli/connect.mdx`, `docs/cli/server.mdx`.

### Globals / registrations hygiene

17. **Annotation hygiene sweep.** **[✅ shipped — commit rquyk]**
    - LuaLS annotations updated to reflect the actual registered
      surface plus everything new from this run. Done in
      `archetect-core/lua/annotations/archetect.lua` and
      `archetect-core/lua/annotations/archetect_modules.lua`.
    - Specific changes:
      - DROPPED stale top-level `env` and `switches` classes (now under
        `archetect.*`).
      - ADDED `Existing.Error` to the `Existing` class.
      - REVISED `format` to list `to_json` / `to_yaml` / `to_toml` as
        canonical, added `from_json` / `from_yaml` / `from_toml`, kept
        `json` / `yaml` / `toml` as `@deprecated` aliases.
      - ADDED top-level `catalog` global with `catalog.render` and
        `CatalogRenderOpts` (was registered but unannotated).
      - ADDED `archetect.model` and `archetect.model.interactive`
        require-module classes with their function surfaces.
      - UPDATED `GitRepo:add` to typed `patterns: string|string[]`.
      - The earlier sessions in this run (prompt return types,
        `Switches`/`Env`, `file` module, `FileScopeOpts`,
        `FileRenderOpts`) had already been added inline.
    - **Doc impact:** the `lua-api.mdx` reference page should mirror
      the annotation files. Use them as ground truth — they're now
      complete and will stay in sync with future source changes
      (since they're shipped via `archetect ide setup`).

### Sequencing note for the doc rewrite

All source-side decisions are now resolved and shipped (or
intentionally deferred). The doc rewrite phases (1–6 above) can now
proceed without waiting on source-side work. **Lua API reference
should be regenerated from the annotation files**, which are now
authoritative — they were swept in commit rquyk and will be the
source of truth for the LuaLS-aware narrative pages.

### Sequencing with the doc rewrite — RESOLVED

All source-side items are now shipped or intentionally deferred:

- **Shipped:** #1, #2, #3, #4 (partial — `add` only), #5, #6, #7, #9,
  #10, #12, #15, #17.
- **Doc-only fixes:** #8 (`prompt_list` docs), #13 (`cache clear`
  description), #14 (`config edit` description), #16 (`connect`/
  `server` positional cleanup).
- **Skipped:** #11 (catalog subcommand — no clear use case yet).

The doc rewrite phases 1–6 can proceed against the current `main-v3`
tip (commits omsmp → uzolp) without further source-side blockers.

---

## Out of Scope

- Docusaurus theming, sidebar ordering, and site-wide navigation are untouched
  beyond removing the rhai-compat sidebar entry.
- Migration guidance for v2 users is its own project
  (`archetect-3-clean-break-migration.md`) and is not folded in here.
- Performance, caching, or rendering-engine changes. Only ergonomic API and
  CLI surface changes that arose from the audit are considered.
