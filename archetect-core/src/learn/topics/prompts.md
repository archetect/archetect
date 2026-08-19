# prompts — seven types, one resolution order

Every input flows through `ctx:prompt_<type>(message, key, opts?)`. The KEY is the contract:
it is what `-a key=value` answers, what `archetect interface` reports, and what the headless
error names when unanswered.

| Type | Returns | Options beyond the shared set |
|---|---|---|
| `prompt_text` | string | `min`/`max` (length), `pattern` (regex), `cases` — enforced on EVERY input path |
| `prompt_int` | integer | `min`/`max` (value) — likewise |
| `prompt_confirm` | boolean | — |
| `prompt_select` | string | `options` (2nd arg), `allow_other`, `other_label` |
| `prompt_multiselect` | string[] | `options` (2nd arg), `default` (string[]), `min`/`max` (item count) |
| `prompt_list` | string[] | `min`/`max` (item count) |
| `prompt_editor` | string | — |

Shared options: `default`, `help`, `placeholder`, `optional` (unanswered → nil instead of
error, and the only way an EMPTY value is accepted), `answer_key` (answer under a different
key), `cases` (case-variant expansion — see `archetect learn cases`), and `ui` (an opaque
metadata table carried to clients untouched). Grouping is not an option — see pages and
sections below.

Select/multiselect `options` entries are bare strings or rich tables
`{ value = "pg", label = "PostgreSQL", help = "Production-grade" }` — the VALUE is the
contract (what `-a` answers, what `default` names, what is stored); labels are display-only.

## Resolution order (same for every type)

1. An **answer** exists for the key (config → `-A` file → `-a` flag; last wins) → used,
   validated against the type and the prompt's declared rules.
2. Defaults apply (`--headless`, `-D`, or `-d <key>`) → the prompt's `default`; if `optional`
   and no default → nil.
3. Otherwise: ask interactively — or under `--headless`, an ERROR that IS the interface:
   `no answer or default for '<message>' — answer key `key` (CLI: -a key=<value>; MCP:
   answers.key)`. Supply that key; re-run.

## Pages and sections: where a long form breaks

A thirty-prompt archetype is a thirty-field wall unless the author says otherwise. `context:page`
and `context:section` are containers around the prompts a body declares:

```lua
context:page("Service Identity", function(ctx)
  ctx:prompt_text("Service Name:", "service_name")
  ctx:section("Ownership", function(ctx) ctx:prompt_text("Team:", "team") end)
end)
context:page({ title = "Persistence", key = "storage", help = "Where state lives." }, function(ctx)
  ctx:prompt_select("Database:", "database", { "none", "postgres" })
end)
```

Same container, different `kind`; archetect carries the distinction and refuses to decide what
either LOOKS like — a wizard step vs a fieldset, both headings on a terminal, comment banners in
`--answers-template`. Nesting is unrestricted. `title` is required; `key` defaults to its slug
(`service_identity`) and is what a wizard routes on, so pin one if the title may be reworded.

Containers change what a render *looks* like, never what it *produces*. They REPLACE the old
per-prompt `group = "Identity"` label — a string with no extent and nothing that rendered it;
passing `group` is now an error naming this syntax.

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
  -a k=v / -A file     # answers you already have — see below
  -s <switch>          # open the prompts a switch gates
```

**Derive what is still UNKNOWN, not everything askable.** `-a`/`-A` drop the answered prompts and
resolve any branch they select — `-a messaging=kafka` returns that branch's prompts directly,
not every branch behind an `appears_when`. So a wizard paginates by describing again with what
it has, and the final render asks nothing. MCP `describe` takes `answers`/`switches`; gRPC
`DescribeArchetype` takes `answers_yaml`/`switches`.

Declared interfaces (`interface:` blocks / sibling `interface.yaml`) are GONE — a manifest
still carrying one is a load ERROR: a declaration is a second copy of what the prompts
already say, and second copies drift. Derive, don't declare.

Go deeper: `archetect learn rendering` (answering from the CLI) · `archetect learn mcp`
(prompts as a turn-based session).
