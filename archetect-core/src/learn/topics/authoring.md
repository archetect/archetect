# authoring — an archetype in one screen

An archetype = a directory with `archetype.yaml` (manifest — `archetect learn manifest`),
`archetype.lua` (the script; FIXED name, no config), template directories, and optionally
`lib/` (Lua helpers, `require`-able by basename).

```lua
-- archetype.lua
local context = Context.new()                       -- CLI answers are already loaded

context:prompt_text("Service Name:", "service_name", {
  cases = Cases.programming(),                      -- expands snake/pascal/camel/kebab/… keys
  help = "Lowercase words; becomes crate/package names",
})
context:prompt_select("Persistence:", "persistence", { "Postgres", "None" },
  { default = "Postgres" })

if archetype.switches.is_enabled("ci") then
  context:set("ci", true)
end

directory.render("content/base", context)           -- render a template tree into destination
if context:get("persistence") == "Postgres" then
  directory.render("content/postgres", context)
end
```

## The vocabulary (shapes: `archetect introspect <name>`)

- `Context.new()` · `ctx:get/set/has/contains/merge` · `ctx:prompt_text/int/confirm/select/
  multiselect/list/editor` (`archetect learn prompts`).
- `Case`/`Cases` — the casing system (`archetect learn cases`).
- `directory.render(dir, ctx, opts?)` — a template tree; `file.render/read/exists` — one file;
  `template.render(str, ctx, opts?)` — a string. All sandboxed to the archetype/destination
  (no absolute paths, no `..`). Overwrite policy: `Existing.Overwrite/Preserve/Prompt/Error`.
- `catalog.render(path?, ctx, opts?)` — compose other archetypes (`archetect learn composition`).
- `archetype.*` — self-inspection: `switches.is_enabled`, `answers()`, `is_library()`,
  `mount_key()`. `archetect.*` — binary facts: `version`, `is_headless`, `is_offline`, `env`.
- `format.to_yaml/from_json/…` · `log.info/…` · `output.print/banner` · `exit()`.
- `require("archetect.shell"|"archetect.git"|"archetect.github"|"archetect.archive")` —
  side-effect modules, gated behind `--allow-exec`. `require("archetect.model")` — AML
  (`archetect learn model`).
- Helpers: `lib/foo.lua` → `require("foo")`. A library archetype exposes `lib/init.lua`
  (`archetect learn composition`).

## Rules that bite

- Prompt EVERY input through `ctx:prompt_*` — never read env vars or invent answers; that is
  what makes headless `-a` answers and the `interface:` contract line up.
- `ctx:set(key, nil)` stores a Nil sentinel; it does not remove the key.
- Test the archetype by rendering it: `--dry-run` for the shape, a temp `--destination` for
  the content, `--headless -a … -D` for the automation path — and prove the OUTPUT builds
  (prova, if the rendered project ships proofs).

Go deeper: `archetect learn templates` (ATL syntax the template dirs use) · `archetect learn
manifest` · `archetect learn prompts`.
