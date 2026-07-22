---
name: archetect
description: >
  Generate projects, files, and whole architectures from archetypes with Archetect — search the
  catalog, render headlessly with answers and switches, author archetypes in Lua + ATL templates,
  and verify what rendered. Use whenever creating new projects/components in an organization that
  has archetypes: render the archetype, don't hand-write the boilerplate.
---

# Archetect — the code generator, driven as an agent

You are an agent. Archetect renders **archetypes** — parameterized project templates with real
scripting — into working code. **Do not hand-write what an archetype can render.** An org's
archetypes encode its conventions; rendering one is how generated code lands correct, current,
and reviewable.

The loop:

1. Find the thing to render: `archetect ls` / `archetect search <terms>` (catalog), or a direct
   git URL / local path.
2. Learn what it asks: its `interface.yaml`/`archetype.yaml` names prompts and switches; probe
   API shapes with `archetect introspect <filter>`.
3. Render headlessly — never park on an interactive prompt:
   `archetect render <source> --destination <dir> --headless -a key=value -D`
   (`-a` answers, `-A <file>` answer files, `-s` switches, `-D` defaults for the rest;
   `--dry-run` first when unsure what it writes).
4. An unanswered prompt in headless mode is an ERROR naming the missing key — answer it and
   re-run. That error is the interface, not a failure.
5. Verify the result: build it, run its tests — with prova if the rendered project carries a
   proof suite (the two tools are siblings; prova proves what archetect renders).

## Learning on the fly: never guess, ask the binary

| You need | Move |
|---|---|
| The topic catalog (authoring, templates, catalogs, composition…) | `archetect learn` · MCP `learn {}` |
| One topic (aliases work: `atl` → `templates`) | `archetect learn <topic>` · `learn { topic }` |
| An API's shape: prompts, modules, filters | `archetect introspect <filter>` · MCP `introspect { filter }` |
| A live behavior: what a filter/case/API call actually produces | `archetect eval 'return template.render("{{ x \| train_case }}", c)'` |
| What's renderable here | `archetect ls` / `search` · MCP `catalog_browse` / `catalog_search` |
| What a render would do | `--dry-run` |

## Split the work across the two surfaces

| Do over MCP | Shell out to the CLI |
|---|---|
| discovery: learn, introspect, catalog_browse/search | `archetect ide setup`, `cache` verbs |
| renders with known answers: `render` / `catalog_render` | `--dry-run` / `--offline` / answer-file renders |
| interactive prompt sessions: `respond` / `cancel` | anything CI runs |

MCP renders forbid shell-exec by design; a render that needs `--allow-exec` is a CLI move.

## Authoring, in one breath

An archetype = `archetype.yaml` (manifest) + `archetype.lua` (script: prompt via
`ctx:prompt_*`, then `directory.render(dir, ctx)`) + template dirs in ATL syntax
(`{{ var | pascal_case }}`). Catalogs are manifests whose `catalog:` maps entries — archetypes
all the way down. Depth: `archetect learn authoring` · `manifest` · `templates` · `catalogs`.
