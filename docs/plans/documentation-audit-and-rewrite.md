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
- **Source reality:** `archetect-core/src/script/lua/modules.rs:908-910`
  registers `Existing.Overwrite`, `Existing.Preserve`, `Existing.Prompt`.
  Default when `if_exists` is omitted is `Preserve`
  (`modules.rs:925`).
- **Action:** Replace `Existing.Skip` → `Existing.Preserve`,
  `Existing.Error` → `Existing.Prompt` everywhere. Update the default
  callout. Affected files (confirmed): `docs/scripting/rendering.mdx:42,89-92`,
  `docs/scripting/index.mdx:54`,
  `docs/reference/lua-api.mdx:21,48,113`.

### 4. `archetect.switches` does not exist

- **Claim (docs):** `archetect.switches:contains(name)`.
- **Source reality:** The `archetect` global exposes `version`,
  `version_major`, `version_minor`, `version_patch`, `answers()` only
  (`modules.rs:187-199`). Switches are on their own top-level global
  registered by `register_switches_module` as `switches.is_enabled(name)`
  (`modules.rs:478-480`).
- **Action:** Global substitution `archetect.switches:contains(X)` →
  `switches.is_enabled(X)`. Affected: `docs/scripting/composition.mdx:111-113`,
  `docs/reference/lua-api.mdx:129`. Re-audit any sidebar summaries.

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

### 6. Method name: `prompt_multi_select` (underscore)

- **Claim (docs):** `context:prompt_multiselect(...)`.
- **Source reality:** Registered as `prompt_multi_select`
  (`archetect-core/src/script/lua/context.rs:608`). The no-underscore spelling
  errors at runtime.
- **Action:** Replace `prompt_multiselect` → `prompt_multi_select` wherever
  it appears. Affected: `docs/scripting/prompting.mdx:59`,
  `docs/reference/lua-api.mdx:36`.

### 7. Prompt methods return nothing

- **Claim (docs):** "The result is also returned, so you can use it inline."
- **Source reality:** All `prompt_*` methods return `Ok(())`. The value is
  stored into context under the key; callers must use `ctx:get(key)` to read
  it back.
- **Action:** Remove all inline-return examples; rewrite to use
  `ctx:prompt_text(...); local v = ctx:get("key")`.

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

### Lua API ergonomics

1. **Prompt methods should return the prompted value.** **[adopt docs-side design]**
   - Current: `ctx:prompt_text(msg, key)` returns `()`; caller must
     `ctx:get(key)` after.
   - Proposed: return the value *and* store it in context. Source change in
     `archetect-core/src/script/lua/context.rs` on every `prompt_*` method:
     return the typed value via `mlua` instead of `Ok(())`.
   - Rationale: every example in foundational-scratch and every doc example
     reads naturally as `local name = ctx:prompt_text(...)`. Forcing a
     second lookup is noise.
   - Migration risk: none — adding a return value is additive.

2. **`Existing` enum: add `Skip` alias for `Preserve`, and add a real `Error` variant.** **[needs decision]**
   - Current: `Existing.Overwrite | Preserve | Prompt` (`modules.rs:908-910`).
   - Problem A (`Skip`): `Preserve` reads as "keep the old file" which is
     fine, but `Skip` is more intuitive for the "don't write" semantic.
     Cheapest fix: register `Existing.Skip` as an alias alongside `Preserve`.
   - Problem B (`Error`): docs assumed a variant that fails the render when
     a file exists — this is a legitimate semantic (safest for idempotent
     CI renders) and is NOT equivalent to `Prompt`. Add a real
     `OverwritePolicy::Error` variant that returns a render error on
     conflict.
   - Rationale: CI use cases want hard-fail-on-conflict; currently the
     closest option is `Prompt`, which hangs in a headless run.

3. **`prompt_multi_select` → `prompt_multiselect` (drop the underscore).** **[adopt docs-side design]**
   - Current: `context.rs:608` registers `prompt_multi_select`.
   - Proposed: register both names, then deprecate the underscored form.
   - Rationale: "multiselect" is the commonly used single word in prompt
     libraries (inquire, prompts, etc.); the inner underscore reads oddly
     next to `prompt_text`, `prompt_int`, etc.

4. **Add `repo:status()` and accept a list/table in `repo:add(...)`.** **[adopt docs-side design]**
   - Current: `repo:add` takes one string; no `status` method
     (`require_modules.rs:358-387`).
   - Proposed: accept `String | Vec<String>` in `add` (iterate if table);
     add `status()` returning `StatusOutput` as a string or a structured
     table with `{ staged, unstaged, untracked }`.
   - Rationale: both are obvious ergonomics wins and were already documented.

5. **Add `__tostring` metamethod on `Context`.** **[adopt docs-side design]**
   - Current: `tostring(ctx)` returns `"userdata: 0x…"`.
   - Proposed: implement `__tostring` to produce a compact
     `key=value,key=value` dump (or full YAML-like).
   - Rationale: debugging a scripted archetype is painful without this;
     `log.debug(tostring(ctx))` is the natural move.

6. **Consolidate `switches` / `env` under `archetect`.** **[needs decision]**
   - Current: top-level `switches` global (`is_enabled(name)`) and separate
     `env` global.
   - Proposed: expose them as `archetect.switches` and `archetect.env`
     while keeping the top-level globals as aliases for back-compat.
   - Rationale: docs-writer intuition clustered runtime info under
     `archetect.*`. That grouping is more discoverable; the flat top-levels
     are fine but not obvious.

7. **Add a `read_file(path)` helper resolved against the archetype root.** **[adopt docs-side design]**
   - Current: no such helper; users drop to `io.open` which resolves against
     process cwd, not the archetype source root.
   - Proposed: register `read_file(relative_path)` that resolves against
     the active archetype's source dir and returns the file contents (or
     errors).
   - Rationale: the rendering doc's example assumed this exists, and it's
     genuinely useful for partials-style workflows.

8. **`prompt_list` should be documented; consider alias `prompt_strings`.** **[keep source, fix docs]**
   - Current: method exists at `context.rs:668` but is undocumented.
   - Proposed: no source change; just document it.

### CLI ergonomics

9. **Accept destination as an optional second positional on `render` / top-level action / `global` / `connect`.** **[adopt docs-side design]**
   - Current: `--destination`/`--dest` only; a bare second word is rejected
     by clap.
   - Proposed: add a positional `[destination]` that falls back to
     `--destination`, defaulting to `.`.
   - Rationale: every doc example assumed this shape, matching v2 ergonomics
     and matching common tooling (`git clone <url> <dir>`,
     `cargo new <name>`, etc.). The friction of typing `--destination` for
     every run is real.

10. **Add `--offline` / `-o` to `ls`.** **[adopt docs-side design]**
    - Current: `ls` has no flags (`cli.rs:72-76`).
    - Proposed: at minimum support `--offline` so `ls` can enumerate the
      local cache without network.
    - Rationale: docs assumed this; the feature is useful and cheap.

11. **`archetect catalog` as an explicit subcommand.** **[needs decision]**
    - Current: there is no `catalog` subcommand; the root `action` positional
      handles catalog browsing.
    - Proposed: either leave as-is and fix the doc, OR add a `catalog` subcommand
      as an explicit alias (e.g. `archetect catalog <path>` equivalent to
      `archetect <path>`) for discoverability in `--help` output.
    - Rationale: catalog browsing is currently hidden from `--help`; new users
      never discover it.

12. **`git.init` default branch should default to `"main"` explicitly.** **[adopt docs-side design]**
    - Current: source omits `-b` and defers to git's `init.defaultBranch`
      config — which is frequently unset or still `master`.
    - Proposed: pass `-b main` by default; allow override via the `branch`
      option.
    - Rationale: deterministic behavior across developer machines. Docs
      already assumed this.

13. **`cache clear`: behavior doesn't match the "equivalent to rm -rf" framing.** **[keep source, fix docs]**
    - The source's per-entry removal with confirmation is safer than a
      blanket `rm -rf`. Docs should be corrected, not source.

14. **`config edit`: document that the file is only written on editor save.** **[keep source, fix docs]**
    - Source's behavior is correct; the doc is wrong.

15. **`ide` subcommand: add `--force` to overwrite an existing `.luarc.json`.** **[adopt docs-side design]**
    - Current: silently skips.
    - Proposed: add `--force` flag; default behavior stays non-destructive
      and now prints a clearer "skipping, use --force to overwrite" message.

16. **`connect` and `server`: remove all doc references to source positionals.** **[keep source, fix docs]**
    - These subcommands are correct as-is; the docs were invented.

### Globals / registrations hygiene

17. **Registered-but-undocumented globals and modules need IDE annotations too.**
    `output`, `runtime`, `env`, `switches`, `format`, `exit`, `archetype`,
    `Case`, plus require modules `archetect.shell`, `archetect.archive`,
    `archetect.model`, `archetect.model.interactive`. Confirm every one has
    corresponding entries in the LuaLS annotations shipped by
    `archetect ide`. If any are missing, update the annotation emitter in
    `archetect-bin/src/subcommands/ide_subcommand.rs` (or wherever the
    annotation source strings live) alongside the doc rewrite.

### Sequencing with the doc rewrite

Source-side changes should land in this order relative to docs work:

- **Before Phase 2 (reference regeneration):** items #1 (prompt return
  values), #3 (multiselect alias), #4 (git repo:add/status), #5
  (Context __tostring), #12 (git.init default branch). These change the
  surface the reference documents.
- **Before Phase 4 (CLI pages):** items #9 (destination positional),
  #10 (`ls --offline`), #15 (`ide --force`). These change the CLI surface
  the CLI pages document.
- **Deferred / still discussing:** items #2 (Existing variants), #6
  (archetect.switches grouping), #7 (read_file helper), #11 (catalog
  subcommand). Docs may need to stay aligned with current behavior until
  these are resolved.

---

## Out of Scope

- Docusaurus theming, sidebar ordering, and site-wide navigation are untouched
  beyond removing the rhai-compat sidebar entry.
- Migration guidance for v2 users is its own project
  (`archetect-3-clean-break-migration.md`) and is not folded in here.
- Performance, caching, or rendering-engine changes. Only ergonomic API and
  CLI surface changes that arose from the audit are considered.
