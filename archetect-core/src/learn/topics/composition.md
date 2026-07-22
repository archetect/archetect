# composition — archetypes rendering archetypes

Composition IS the catalog mechanism: an archetype's script renders other entries, and the
flag bags flow down. There is no separate "component" system to learn.

```lua
-- inside archetype.lua
local child = catalog.render("services/grpc", context, {
  destination = "services/" .. context:get("service_name"),
  switches = { "ci" },              -- overlay ON TOP of inherited switches
})
context:merge(child)                -- a child returns its resulting context — take what you need
```

- `catalog.render(path?, ctx, opts?)` — no path → present this archetype's own `catalog:`
  menu; a leaf path → render it; opts: `destination`, `switches`, `use_defaults`,
  `use_defaults_all`.
- Flag propagation, three layers, most-specific last: inherited (parent's switches/defaults)
  → `opts` on the call → the ENTRY's own `answers`/`switches`/`use_defaults(_all)`.
  Same `name` adds / `name=false` removes semantics everywhere.
- The child runs against the same answer set — a child prompt already answered at the top
  level never re-asks. Namespace child-specific keys (`-a billing.port=…` nests dotted keys).

## Libraries — shared Lua without copy-paste

A **library archetype** ships reusable Lua instead of (or besides) content: its main module
at `lib/init.lua`, consumers declare it as a catalog entry with `library: true`, and the
entry KEY becomes the module name:

```yaml
catalog:
  scm: { source: "https://github.com/acme/scm-library.git#v1", library: true, show: false }
```

```lua
local scm = require("scm")          -- the entry key; staged before your script runs
scm.setup(context)
```

- Staging is eager: `lib/` + `includes/` land on the consumer's `package.path` at load.
- A library self-tests standalone: its own `archetype.lua` shim exercises it.
- Inside a library, `archetype.is_library()` / `archetype.mount_key()` /
  `archetype.include_path(rel)` answer "how am I mounted?" — templates a library renders
  must address includes through `include_path` so they resolve under any mount key.

## Where things live

| Need | Reach |
|---|---|
| Helpers private to one archetype | `lib/foo.lua` → `require("foo")` |
| Shared prompts/util across an org | a library archetype, `library: true` entry |
| A whole sub-project | a leaf entry + `catalog.render` |
| Sequenced multi-archetype builds | a parent archetype whose script renders entries in order, threading `context` |

Go deeper: `archetect learn catalogs` (the entry schema) · `archetect learn model`
(composition driven by a domain model).
