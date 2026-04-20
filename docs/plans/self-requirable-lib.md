# Self-requirable `lib/init.lua` for Library Archetypes

## Status

Drafted 2026-04-20. Not started.

| Phase | Scope | Status |
|---|---|---|
| 1 | Add `<root>/?.lua` + `<root>/?/init.lua` to `package.path` | pending |
| 2 | Test coverage (self-require fixture) | pending |
| 3 | Doc update in `docs/scripting/libraries.mdx` | pending |

## Motivation

The `-library` convention in the v3 ecosystem design has libraries
expose their main module at `lib/init.lua`. Consumers who mount the
library with `library: true` reach it as `require("<map-key>")` —
e.g. `require("scm")` when the catalog key is `scm` — because the
library's `lib/` is symlinked under `<staging>/lib/<map-key>/` and
`<staging>/lib/?/init.lua` is on `package.path` (see
`archetect-core/src/script/lua/modules.rs:129-141`).

When the same library is invoked **standalone** (its own
`archetype.lua` shim runs as a one-shot entry point), there is no
map-key context. Only the library's own `lib/` is on `package.path`,
via `<root>/lib/?.lua` + `<root>/lib/?/init.lua`
(`modules.rs:122-127`). With those patterns, the only way to reach
`<root>/lib/init.lua` is `require("init")` — because `init` is the
basename of the file and matches the `?.lua` slot.

Concrete example: `archetect-common/scm-library` has
`lib/init.lua` exposing `prompt / finalize / run`. The library's own
shim must do:

```lua
-- archetype.lua
require("init").run(Context.new())
```

Consumers of the same library write:

```lua
-- parent archetype.lua
local scm = require("scm")
scm.prompt(ctx); directory.render(...); scm.finalize(ctx)
```

Same file, two different `require` names. The asymmetry is an
accident of `package.path` plumbing, not a design choice, and every
library author who adopts this pattern hits it.

## Design

Prepend two entries to `package.path` in `register_lua_libraries`, at
the archetype root (not the `lib/` subdirectory):

```
<root>/?.lua
<root>/?/init.lua
```

With those on the path, `require("lib")` resolves to
`<root>/lib/init.lua` via the `<root>/?/init.lua` pattern (Lua
substitutes `lib` for `?` and finds the file). The shim becomes:

```lua
require("lib").run(Context.new())
```

Consumer-side `require("scm")` continues to resolve as today via
`<staging>/lib/scm/init.lua`. Nothing about consumer-facing names
changes.

### Why `lib`

- It's the literal directory name. No invented magic word.
- Reads naturally in a shim: "this library's own lib module".
- Reserves a clear visual distinction between `require("lib")`
  (self) and `require("<map-key>")` (a dependency) — the self/dep
  boundary becomes grep-able.
- No manifest field, no validation, no missing-declaration failure
  mode. Convention-only.

### Rejected alternative: manifest-declared canonical name

Considered a `library.name: scm` manifest field with archetect
mounting the local `lib/` under that declared name. Rejected because:

- Introduces a new manifest field with defaulting rules, validation,
  and a failure mode when absent or out-of-sync.
- Adds a second name to reason about alongside the consumer-chosen
  catalog map key. With the `lib` convention there is one self-name
  globally, and it's the same in every archetype.
- The ergonomics win is negligible: `require("scm")` vs
  `require("lib")` — both are one short word.

## Implementation

Single change site:
`archetect-core/src/script/lua/modules.rs:115-156`
(`register_lua_libraries`). Inside the function, insert two more
segments **before** the existing "Consumer's own lib/" block so the
archetype root is searched before `lib/` — this keeps the existing
`require("helpers") → <root>/lib/helpers.lua` resolution working
while also allowing `require("lib") → <root>/lib/init.lua`.

Sketch:

```rust
// 0. Archetype root — enables `require("lib")` to resolve to
//    <root>/lib/init.lua for libraries that follow the
//    `lib/init.lua` main-module convention. Also makes any
//    top-level directory with an init.lua requirable by its
//    directory name.
let root = archetype.root();
prepend_segments.push(format!("{}/?.lua", root));
prepend_segments.push(format!("{}/?/init.lua", root));

// 1. Consumer's own lib/ — implicit local helpers. (existing)
// 2. Staged library lib dirs. (existing)
```

Update the doc-comment above the function (`modules.rs:100-114`) to
cover the new entry as point 0.

## Side effects

With `<root>/?/init.lua` on the path, any top-level directory in an
archetype that contains an `init.lua` becomes requirable by its
directory name — e.g. a `foo/init.lua` would be reachable as
`require("foo")`. In practice no archetype ships `init.lua` at paths
like `contents/`, `includes/`, `archive/`, etc., so this is a
theoretical hazard, not a real collision risk.

Callout in docs: "Archetype top-level directories are requirable if
they contain `init.lua`. In practice only `lib/` does."

## Scope: self-require only, not consumer-side

This plan makes `require("lib")` work from the archetype's own
script execution. It does **not** extend to consumers.

When a consumer mounts a library with `library: true`, archetect
symlinks only the library's `lib/` under `<staging>/lib/<map-key>/`.
Sibling top-level directories in the library (`prompts/`,
`providers/`, etc.) are not on the consumer's `package.path`. A
library whose `lib/init.lua` did `require("prompts")` would work
when the library runs its own shim, but fail the moment a consumer
`require`'d it — the consumer's Lua state has no search entry that
resolves `prompts` against the library's root.

Practical rule for library authors:

- All internal modules live under `lib/` (e.g. `lib/prompts.lua`,
  `lib/providers/github.lua`).
- Cross-reference them via `require("lib.prompts")` (resolves
  locally) or use the `local mod_name = ...` varargs trick so the
  prefix adapts to whatever name the caller used.

This is a deliberate scoping choice. Libraries in the
`archetect-common/` ecosystem are helpers / orchestrators, not
arbitrary filesystems — the `lib/` boundary is enough. Expanding
library staging to honor a manifest-declared `exports:` list (or
symlink the whole root under the map-key) was considered and
rejected as overkill for what these things actually do.

## Testing

Extend the local-lib test suite. Existing fixture:
`archetect-core/tests/utils/lua_local_lib_tests/` (see
`lua_local_lib_tests.rs` — requires `greet` and `nested.util`
from `<root>/lib/`). Add:

- `tests/utils/lua_self_require_lib_tests/lib/init.lua` — exports a
  table `{ hello = function() return "hi from self-lib" end }`.
- `tests/utils/lua_self_require_lib_tests/archetype.lua` — does
  `print(require("lib").hello())`.
- `tests/utils/lua_self_require_lib_tests/archetype.yaml` — minimal
  manifest.
- `tests/utils/lua_self_require_lib_tests.rs` — asserts the printed
  output, mirroring the structure of
  `lua_local_lib_tests.rs`.

One positive test is sufficient; the failure mode if the path
entry is missing is `require("lib")` raises "module 'lib' not
found", which the test catches via render-succeeded + expected
print.

## Docs

Add a "self-require" subsection to `docs/scripting/libraries.mdx`
(or the equivalent in the Docusaurus site if that audit from the
documentation-audit plan has landed):

- Libraries that expose a main module via `lib/init.lua` can reach
  their own module as `require("lib")` from the archetype's own
  script (typically the one-shot shim in `archetype.lua`).
- Consumers reach the same module as `require("<map-key>")` via the
  staged-library wiring — the names differ because consumers choose
  their map key.
- Show the canonical shim pattern:

  ```lua
  -- archetype.lua (library's one-shot entry point)
  require("lib").run(Context.new())
  ```

Cross-reference `docs/specs/v3-ecosystem-design.md` point 9
("Standardized exports paths") — this plan extends that standard by
making the self-name predictable too.

## Relationship to other docs

- **`docs/specs/v3-ecosystem-design.md`** — canonical `-library`
  convention and `lib/` + `includes/` export paths. This plan adds
  the self-require corollary.
- **`docs/plans/archetect-3-lua-scripting-engine.md`** — the
  broader Lua engine work; this is a small follow-on refinement.
- **`archetect-common/scm-library`** — the first library to hit this
  asymmetry; will drop `require("init")` in favor of
  `require("lib")` once this ships. Grep for `require("init")` at
  cutover time — `scm-library/archetype.lua` is the only known site.
