# manifest — archetype.yaml, every key

One manifest format serves archetypes AND catalogs (`archetype.yaml`; `archetype.yml` /
`archetect.yaml` / `.yml` also load). Unknown keys are ignored (forward-compat) — a typo'd
key silently does nothing, so check spelling here first when a setting "doesn't work".

| Key | Meaning |
|---|---|
| `description` / `summary` | what this is; `summary` feeds search listings |
| `authors`, `languages`, `frameworks`, `tags` | metadata; `search` matches all of these |
| `requires.archetect` | version gate: majors are walls (a 2.x archetype refuses a 3.x binary with a "use archetect2" error); within a major it is a minimum floor |
| `templating.undefined` | `lenient` (default) or `strict` — strict makes an undefined `{{ var }}` a render ERROR; turn it on, it catches typos |
| `templating.trim_blocks` / `lstrip_blocks` | whitespace control for block tags |
| `catalog` | ordered map of entries — presence of entries + no `archetype.lua` makes this a CATALOG; see `archetect learn catalogs` |
| `interface` | declarative prompt/switch contract (or a sibling `interface.yaml`, which OVERRIDES the inline one); see `archetect learn prompts` |

## What is NOT configured here

- The script entry point: always `archetype.lua` at the root.
- Lua helpers: always `lib/`.
- Template content: addressed by root-relative path in `directory.render("content/base", …)`;
  the archetype's own `includes/` is available to templates automatically.

## The catalog entry, since it lives in this file

Each `catalog:` entry is EXACTLY one of: `source:` (leaf → renders an archetype), nested
`catalog:` (group → submenu), `server:` (federation → children fetched from a remote
archetect server). Plus per-entry `description`, `answers`, `switches`, `use_defaults`,
`use_defaults_all`, `library: true` (stage its `lib/` into consumers), `show: false` (hide
from menus, still addressable). Mixing kinds in one entry is a load error.

Go deeper: `archetect learn catalogs` (entries in practice) · `archetect learn prompts` (the
`interface:` block) · `archetect learn templates` (what `templating:` governs).
