# archetect-common API Polish (3.0.0)

Review of all archetypes and libraries in `archetect-common` (and the wider
`archetect-rust` / `archetect-aml` repos) against the v3 Lua API design.
Goal: resolve every design inconsistency and fix every bug before tagging 3.0.0.

Repos touched by this plan:
- `archetect-common` — the primary target (libraries + starters)
- `archetect-rust` — downstream consumer, drives some of the design decisions
- `archetect-3` — engine; annotation files and any API surface gaps belong here

---

## Section 1 — Design Decisions (resolve before writing any fixes)

These are open questions where the answer shapes everything downstream.

### 1.1 Composition Paradigm: `require()` two-phase vs `catalog.render()` + merge

**DECIDED:** Both patterns are valid and supported. Use the right one for the situation.

**Model A — `require()` + two-phase** (use when you need phase separation):
```lua
local scm = require("scm")
scm.prompt(context)
directory.render("contents", context)  -- can reference scm answers
scm.finalize(context)                  -- side effects happen last
```

**Model B — `catalog.render()` one-shot** (use when phase separation isn't needed):
```lua
context:merge(catalog.render("author-prompts", context))
context:merge(catalog.render("license", context, { destination = project_dir }))
directory.render("contents", context)
```

**Engine behavior confirmed:** `catalog.render(key, context, opts)` extracts the
parent context as a `ContextMap` and passes it to the child's `RenderContext` as
pre-seeded answers. `Context.new()` in the child script automatically loads those
answers — so `context:get("repo_name")` finds the parent's value and the prompt
is skipped. The returned value is a fresh context containing only the child's
output; the parent merges it with `context:merge()`.

Precedence: parent answers → catalog entry `answers:` overrides → prompt defaults → interactive

**Rule:**
- **`require()` + two-phase:** use when you need to render content between prompt and
  finalize (i.e. your rendered templates reference answers that affect what gets
  finalized — e.g. scm-library must know what it's git-init-ing after rendering)
- **`catalog.render()` + `context:merge()`:** valid one-shot for all libraries when
  no interleaving is needed. Parent answers are properly seeded into the child.

Both patterns are supported and correct. Authors choose based on whether they need
phase separation.

- [x] Decision recorded
- [x] Engine behavior confirmed (parent context seeds child via pre-seeded answers)
- [x] Document both patterns in each library's README — added `catalog.render()` one-shot
  sections to all 6 libraries (gitignore, license, author-prompts, editor-config,
  org-prompts, project-prompts). scm-library was already complete.

### 1.2 `catalog.render()` / `component.render()` / `archetype.render_component()` — pick one name

**DECIDED:** `catalog.render()` is the canonical name.

Rationale: most widely used already, maps directly to the `catalog:` key in
`archetype.yaml`, self-documenting ("you're rendering something from your catalog").

- [x] Decision recorded
- [x] Update LuaLS annotations to document `catalog.render(key, context, opts?)` — already present
- [x] Update archetect-aml to replace `component.render()` with `catalog.render()`

### 1.3 `cases` option shape — flat list vs nested

Three syntaxes appear:
```lua
cases = Cases.programming()                                       -- form A
cases = { Cases.programming() }                                   -- form B (wrong if A returns list)
cases = { Cases.programming(), Cases.fixed("x", Case.Title) }    -- form C
```

`Cases.programming()` returns a `CaseSpec[]`. `Cases.fixed()` returns one `CaseSpec`.
Form B wraps a list inside a list (wrong). Form C mixes a list and a scalar in one
table (wrong). The only consistent interpretation: `cases` is `CaseSpec[]`, and the
correct forms are:

```lua
cases = Cases.programming()                           -- spread of built-in set
cases = Cases.fixed("prefix_title", Case.Title)      -- single spec, also a valid list of 1?
cases = { Cases.fixed("a", Case.Title),
          Cases.fixed("b", Case.Snake) }             -- explicit multi-spec list
```

Or `Cases.programming()` could be designed to return a single opaque `CaseSpec`
(not a list), making all three forms equivalent. But that loses the ability to
combine it with `Cases.fixed()` in one call.

**DECIDED:**
- `Cases.programming()` returns a flat `CaseSpec[]` (the standard programming set)
- `Cases.fixed(key, case)` returns a single `CaseSpec`
- `cases` option accepts a `CaseSpec[]`
- To use just the programming set: `cases = Cases.programming()`
- To add a fixed key on top: build the list explicitly or use a helper

The wrapping forms `{ Cases.programming() }` and
`{ Cases.programming(), Cases.fixed(...) }` are wrong and must be fixed.
The correct combined form is:

```lua
-- Only fixed keys:
cases = { Cases.fixed("prefix_title", Case.Title) }

-- Combining programming set with a fixed key requires spreading or a helper.
-- Proposal: Cases.extend() or just document "use Cases.programming() alone,
-- add fixed keys in a separate context:set() if needed".
```

**Engine confirmed (archetect-core/src/script/lua/cases.rs):**
- `Cases.programming()` / `Cases.all()` / `Cases.set()` return `CaseSpecList` (a list)
- `Cases.fixed()` / `Cases.input()` return a single `CaseSpecEntry`
- The `cases` option's `extract_cases()` accepts a single item OR a Lua table;
  tables are flattened — `CaseSpecList` elements are spread, `CaseSpecEntry` elements
  are pushed individually
- **No `Cases.extend()` needed** — the Lua table is the composition mechanism:

```lua
cases = Cases.programming()                                          -- list only
cases = Cases.fixed("prefix_title", Case.Title)                     -- single fixed
cases = { Cases.programming(), Cases.fixed("prefix_title", Case.Title) }  -- combined
```

`{ Cases.programming() }` (wrapping a list in a redundant table) works but is
noise — simplify to `Cases.programming()` directly.

- [x] Decision recorded
- [x] Engine behavior confirmed — no Cases.extend() needed
- [x] Fixed all `{ Cases.programming() }` redundant wrapping call sites
- [x] Update LuaLS annotations to document `CaseSpecList`, `CaseSpecEntry`, `Case.*` enum — all present; unified `CaseSpec` opaque type is correct for LuaLS

### 1.4 `Existing.*` and `Location.*` — enum constants vs strings

The engine exposes these as global table constants:
```lua
if_exists = Existing.Overwrite   -- vs "overwrite"
within = Location.Destination    -- vs "destination"
```

The design-doc LuaLS annotations show string literals. Enum constants are better
(typos become nil-dereference errors, LuaLS can enumerate valid values), but
they need to be documented.

**DECIDED:** Keep enum constants. Better IDE experience — typos become nil errors,
LuaLS can enumerate valid values.

- [x] Decision recorded
- [x] Add `Existing`, `Location`, and `Case` enums to LuaLS annotations in archetect-3 — all present

---

## Section 2 — Bugs (wrong behavior, fix immediately)

### 2.1 `library-starter` and `catalog-starter` produce mixed-notation repo names

**Files:**
- `library-starter-archetype/archetype.lua:66`
- `catalog-starter-archetype/archetype.lua:37`

```lua
-- BUG — uses snake variant, produces "my_service-library":
local repo_name = context:get("archetype_name") .. "-library"

-- FIX — use kebab variant, produces "my-service-library":
local repo_name = context:get("archetype-name") .. "-library"
```

Same fix in `catalog-starter` (`.. "-catalog"`).

The generated `archetype.yaml` and `lib/init.lua` templates use `{{ archetype_name }}`
for directory names inside the repo, which is correct (template directories can use
any case variant). The bug is only in the _repo name_ derivation at lines 66 / 37.

- [x] Fix `library-starter-archetype/archetype.lua`
- [x] Fix `catalog-starter-archetype/archetype.lua`

### 2.2 `rust-workspace-scaffolding.archetype3` sets wrong gitignore key

**File:** `archetect-rust/rust-workspace-scaffolding.archetype3/archetype.lua:34`

```lua
-- BUG — gitignore-library reads "ignores", not "ignore":
context:set("ignore", {"IDEA", "VSCode", "Eclipse", "Claude", "Rust"})

-- FIX:
context:set("ignores", {"IDEA", "VSCode", "Eclipse", "Claude", "Rust"})
```

Silent failure: the library always prompts (sees no pre-set `ignores`) regardless
of the caller's intent.

- [x] Fix `archetect-rust/rust-workspace-scaffolding.archetype3/archetype.lua`

---

## Section 3 — Polish (API consistency, user experience)

### 3.1 `log.info` used for user-facing output

`log.*` is developer-level (may be filtered by log level). User-facing messages
that are always meant to be visible should use `output.print`.

**Affected files and lines:**

| File | Message |
|------|---------|
| `archetype-starter-archetype/archetype.lua:107–109` | "Local archetype created at…" |
| `library-starter-archetype/archetype.lua:152–154` | "Local library created at…" |
| `catalog-starter-archetype/archetype.lua:107–109` | "Local catalog created at…" |
| `scm-library/lib/init.lua:82–88` | "To publish manually:" + instructions |
| `archetect-rust/rust-seaorm-service-builder-archetype/archetype.lua:88–93` | "Next steps:" |
| `archetect-rust/rust-clap-cli-archetype/archetype.lua:52–57` | "Next steps:" |

The scm-library block is the worst: it interleaves `output.print("")` and
`log.info("To publish manually:")` on consecutive lines.

- [x] Audit all `log.info` calls for user-facing intent and convert to `output.print`.
  Fixed: archetype-starter, library-starter, catalog-starter, scm-library,
  rust-seaorm-service-builder-archetype, rust-clap-cli-archetype, aml-builder.

### 3.2 `return context` inconsistency in archetype entry points

Archetype scripts that are composed via `catalog.render()` must return a context
so the caller can merge new keys. Scripts that are top-level (never composed) don't
need to. Currently this is accidental.

Rule: if an archetype's `archetype.lua` is ever the target of `catalog.render()`,
it must `return context`. Starters are never composed — they don't return.
Libraries' standalone shim (`require("lib").run(context)`) does return.

Audit and make it intentional with a comment where present.

- [x] Audit complete. All library/component archetypes return context ✓. All top-level
  starters do not ✓. Bonus: found and fixed `ignore`→`ignores` bug in
  rust-mcp-sdk-server.archetype3 and rust-rmcp-mcp-server.archetype3; fixed
  additional `log.info` → `output.print` in rust-rmcp-mcp-server.archetype3.

### 3.3 `github/` path convention differs between starters

`archetype-starter` puts GitHub workflow templates under `contents/github/`;
`library-starter` puts them at the root `github/` directory.

Pick one location for consistency. `github/` at root is cleaner (it's not content
rendered into the new repo's tree, it's scaffolding for the repo's CI — keeping it
separate from `contents/` makes the distinction explicit).

- [x] Aligned all three starters to root-level `github/`. Also fixed snake→kebab in
  all template directory names (`{{ archetype_name }}` → `{{ archetype-name }}`) across
  library-starter and catalog-starter contents/ and github/ subdirectories.

### 3.4 `context:merge()` — document or remove from public API

`context:merge(other)` appears in archetect-rust but is absent from LuaLS
annotations and archetect-common. If it's a supported method, add it to
annotations. If it only makes sense alongside `catalog.render()` as a composition
tool, document that pairing explicitly.

- [x] Add `context:merge()` to LuaLS annotations in archetect-3 — already present.

### 3.5 `prompt_select` `allow_other` option — document

`project-prompts-library/lib/init.lua:48` uses `allow_other = true/false` on
`prompt_select`. This is a useful escape-hatch option (adds a free-text entry to
a curated list) and should be in the LuaLS annotations.

- [x] Add `allow_other` to `prompt_select` opts in LuaLS annotations — already present.

### 3.6 `archetype.is_library()` called in `M.prompt()` (editor-config-library)

**File:** `editor-config-library/lib/init.lua:92`

`M.prompt()` is supposed to be pure context — no side effects. But it calls
`archetype.is_library()` and conditionally populates `components.editor_config`.
That component publication is effectively a side effect that happens inside the
prompt phase.

This works in practice (mount state is static), but it muddies the two-phase
contract. Either move the component publication to `M.finalize()`, or acknowledge
it in a comment as an intentional exception (the component map must be available
before finalize so parents can use it during their own render phase).

- [x] Left in `M.prompt()` by design. Tightened comment to explain why: component
  publication is context-building (not a filesystem side effect), and parents need
  `components.editor_config` available before their `directory.render()` call, which
  happens between prompt and finalize.

### 3.7 Jinja2 template expression in `catalog-starter` placeholder

**File:** `catalog-starter-archetype/archetype.lua:13`

```lua
placeholder = "{{ description | kebab_case }}",
```

A placeholder string containing Jinja2 syntax. It's unclear whether this is
rendered at prompt time (showing the dynamically derived value) or shown literally
(`{{ description | kebab_case }}`). If the latter, it's misleading. If the former,
it requires the engine to template-render placeholder strings — which is
non-obvious behavior worth a comment or a cleaner approach.

- [x] Placeholder strings are not template-rendered by the engine. Fixed by replacing
  `placeholder = "{{ description | kebab_case }}"` with
  `default = template.render("{{ description | kebab_case }}", context)` — the derived
  value now pre-fills the prompt default using the previously collected description.

---

## Section 4 — LuaLS Annotation Gaps (archetect-3 engine task)

After reviewing `archetect-core/lua/annotations/archetect.lua` and
`archetect_modules.lua`, most items were already present. Annotation files
are comprehensive and well-written.

- [x] `context:merge(other: Context)` — already present (line 46)
- [x] `catalog.render(key, context, opts?)` — already present with `CatalogRenderOpts`
- [x] `Cases.*` return types — present; `CaseSpec` used as unified opaque type (correct for LuaLS usability)
- [x] `Case.*` enum — already present with all 15 styles
- [x] `Existing.*` enum — already present with Overwrite/Preserve/Prompt/Error
- [x] `Location.*` enum — already present with Archetype/Destination/Cwd
- [x] `prompt_select` `allow_other` — already present in `SelectPromptOpts`
- [x] `archetype.is_library()` — already present
- [x] `Cases.input(key)` — was missing; added to `archetect.lua` annotations

---

## Section 5 — Second Adversarial Audit Findings

A second audit (Lua API edge cases + archetype.yaml manifest surface) produced the
following items. Engine source consulted for each.

### 5.1 `component.render()` still used in aml-builder (§1.2 follow-through)

**File:** `archetect-aml/aml-builder.archetype3/archetype.lua:56`

`component.render()` survived the §1.2 decision to canonicalize `catalog.render()`.
Also found a stray `log.info("Cross-service references:")` on line 96.

- [x] Replace `component.render(archetype_name, context, {...})` with `catalog.render(...)`
- [x] Fix `log.info("Cross-service references:")` → `output.print(...)`

### 5.2 `context:set()` with `nil` stores `ContextValue::Nil` — keys cannot be unset

**Engine:** `archetect-core/src/script/lua/context.rs`

Calling `context:set("key", nil)` does not remove the key — it stores a
`ContextValue::Nil` sentinel. A subsequent `context:get("key")` returns `nil`
(Lua nil), and `context:has("key")` behavior depends on whether `Nil` is treated
as present or absent. There is no `context:remove()` method.

Impact: authors who expect `context:set("key", nil)` to clear a key may silently
produce unexpected downstream behavior.

- [x] Document in LuaLS annotations: `Context:set()` nil stores a Nil sentinel, not
  a removal. Added note to annotation.
- [ ] Consider adding `context:remove(key)` to the engine API.

### 5.3 Case key collision: original key overwritten by its own transform

**Engine:** `archetect-core/src/script/lua/context.rs` `set_with_cases()`

When `context:set("foo_bar", value, { cases = Cases.programming() })` runs, the
engine derives all programming case variants from `"foo_bar"`. One of those variants
is snake_case of `"foo_bar"` which is `"foo_bar"` — the same key. The original value
is therefore overwritten by its own transform result (which should be identical, but
may differ if the input was already in mixed case, e.g. `"fooBar"` → snake =
`"foo_bar"`, original key `"fooBar"` is not overwritten, but the snake variant IS
stored under `"foo_bar"` and the original `"fooBar"` remains). In practice this is
only a footgun when the caller uses a key that happens to match a derived case name.

- [x] Document the behavior: added to `Context:set()` annotation — "use the
  canonical form of the key (kebab or snake) to avoid collision with a derived variant."

### 5.4 `catalog.render()` returns only child-created keys, not parent-seeded keys

**Engine behavior confirmed:**

`catalog.render()` returns a fresh `Context` containing only the keys the child
script explicitly set. Parent answers that were seeded into the child (and merely
read, not re-set) are NOT present in the returned context. If a parent merges the
return value it gets only the child's net-new output.

This is correct and intentional, but surprising to authors who expect
`context:merge(catalog.render(...))` to be a no-op if the child only reads.

- [x] Add a note to the LuaLS annotations: done — `catalog.render()` doc updated
  with explicit "contains only keys the child *wrote*" warning.

### 5.5 Unknown fields in `archetype.yaml` silently ignored

**Engine:** `archetect-core` manifest deserialization uses serde without
`#[serde(deny_unknown_fields)]`.

A typo like `catlog:` or `desciption:` silently produces an empty/default value
rather than an error. Authors lose hours debugging "why isn't my catalog loading?".

- [ ] Consider adding `#[serde(deny_unknown_fields)]` to the manifest structs, or a
  post-parse validation pass that warns on unrecognized top-level keys.

### 5.6 `answer_key` is lookup-only, not storage aliasing — document clearly

**Engine:** `answer_key` in prompt opts tells the engine which context key to look
up for a pre-existing answer (skip-prompt path), but the prompt's own answer is
always stored under the prompt's primary key (not `answer_key`). It is a read alias,
not a write alias.

Authors who expect `answer_key = "alt"` to _store_ under `"alt"` will be surprised.

- [x] Add a clear note to the LuaLS annotation for `answer_key`: done — all prompt
  opts classes updated with "Pre-answer lookup alias only" wording.

### 5.7 `{% if var %}` in strict mode errors on undefined var

**Template engine:** MiniJinja in strict/undefined-is-error mode raises an error if
`var` is not in the context when evaluating `{% if var %}`. The workaround is
`{% if var is defined %}` or `{% if var | default(false) %}`.

This is MiniJinja-standard behavior, but the ATL docs don't call it out and
archetect authors coming from Jinja2 may be surprised (Jinja2 is lenient by default).

- [ ] Document in the ATL reference: "undefined variables raise an error in strict
  mode; use `{% if var is defined %}` or the `| default(filter)` to guard optional
  variables."

---

## Work Order

1. Resolve design decisions (§1.1–1.4) — these gate everything else
2. Fix bugs (§2.1–2.2) — small, unambiguous, do now
3. Polish items (§3.1–3.7) — after design decisions are locked
4. LuaLS annotation updates (§4) — last, after all API surface is confirmed
5. Second audit findings (§5) — documentation and minor fixes
