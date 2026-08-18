# prompts — seven types, one resolution order

Every input flows through `ctx:prompt_<type>(message, key, opts?)`. The KEY is the contract:
it is what `-a key=value` answers, what `archetect interface` reports, and what the headless
error names when unanswered.

| Type | Returns | Options beyond the shared set |
|---|---|---|
| `prompt_text` | string | `min`/`max` (length), `pattern` (regex, ENFORCED on every path), `cases` |
| `prompt_int` | integer | `min`/`max` (value) |
| `prompt_confirm` | boolean | — |
| `prompt_select` | string | `options` (2nd arg), `allow_other`, `other_label` |
| `prompt_multiselect` | string[] | `options` (2nd arg), `default` (string[]), `min`/`max` (item count) |
| `prompt_list` | string[] | `min`/`max` (item count) |
| `prompt_editor` | string | — |

Shared options: `default`, `help`, `placeholder`, `optional` (unanswered → nil instead of
error), `answer_key` (answer under a different key), `cases` (case-variant expansion — see
`archetect learn cases`), `group` (UI section label) and `ui` (opaque metadata table) —
both pure metadata, carried to clients (MCP envelopes, future interface probes) untouched.

Select/multiselect `options` entries are bare strings or rich tables
`{ value = "pg", label = "PostgreSQL", help = "Production-grade" }` — the VALUE is the
contract (what `-a` answers, what `default` names, what is stored); labels are display-only.

## Resolution order (same for every type)

1. An **answer** exists for the key (config → `-A` file → `-a` flag; last wins) → used,
   validated against the type.
2. Defaults apply (`--headless`, `-D`, or `-d <key>`) → the prompt's `default`; if `optional`
   and no default → nil.
3. Otherwise: ask interactively — or under `--headless`, an ERROR that IS the interface:
   `no answer or default for '<message>' — answer key `key` (CLI: -a key=<value>; MCP:
   answers.key)`. Supply that key; re-run.

## Pages and sections: where a long form breaks

A thirty-prompt archetype renders as a thirty-field wall unless the author says otherwise.
`context:page` and `context:section` are how the script says it — containers around the
prompts a body declares:

```lua
context:page("Service Identity", function(ctx)
  ctx:prompt_text("Service Name:", "service_name")
  ctx:section("Ownership", function(ctx)
    ctx:prompt_text("Team:", "team")
  end)
end)

context:page({ title = "Persistence", key = "storage", help = "Where state lives." }, function(ctx)
  ctx:prompt_select("Database:", "database", { "none", "postgres" })
end)
```

The two verbs are the same container with a different `kind`; archetect carries the
distinction and refuses to decide what either LOOKS like. A wizard makes a page a step and a
section a fieldset; the terminal makes both headings; `--answers-template` makes them comment
banners. Nesting is unrestricted — sections hold sections, and a page inside a page is not an
error.

`title` is required. `key` defaults to a slug of it (`"Service Identity"` → `service_identity`)
and is what a wizard routes on, so pin one if the title may be reworded. `help` and `ui` behave
as they do on prompts.

They cost nothing to adopt: containers change what a render *looks* like, never what it
*produces*, and an archetype that declares none behaves exactly as before. The older per-prompt
`group` opt still works and is unaffected.

## The derived interface: ask the archetype, don't trust a file

The prompts ARE the interface — `archetect interface <source>` derives the whole contract
by probing the script (writes discarded, exec forbidden): every prompt's envelope, the
switch names it consults via `is_enabled` (never prompted, so this is their only discovery
path), a `layout` tree of the pages and sections above, and a computed batch/interactive
classification. `layout` is always present — an archetype with no containers gets bare prompt
nodes, so a client has one code path either way.

```
archetect interface <source|catalog-path>   # human summary
  --json               # for tooling (same shape MCP `describe` returns)
  --answers-template   # fill-in YAML for a zero-prompt `-A` render
  --explore            # fork select/confirm branches: conditional prompts + appears_when
```

Declared interfaces (`interface:` blocks / sibling `interface.yaml`) are GONE — a manifest
still carrying one is a load ERROR: a declaration is a second copy of what the prompts
already say, and second copies drift. Derive, don't declare.

Go deeper: `archetect learn rendering` (answering from the CLI) · `archetect learn mcp`
(prompts as a turn-based session).
