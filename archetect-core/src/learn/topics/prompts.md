# prompts — seven types, one resolution order

Every input flows through `ctx:prompt_<type>(message, key, opts?)`. The KEY is the contract:
it is what `-a key=value` answers, what `interface:` declares, and what the headless error
names when unanswered.

| Type | Returns | Options beyond the shared set |
|---|---|---|
| `prompt_text` | string | `min`/`max` (length), `cases` |
| `prompt_int` | integer | `min`/`max` (value) |
| `prompt_confirm` | boolean | — |
| `prompt_select` | string | `options` (2nd arg), `page_size`, `allow_other`, `other_label` |
| `prompt_multiselect` | string[] | `options` (2nd arg), `defaults`, `min`/`max` (item count), `page_size` |
| `prompt_list` | string[] | `min`/`max` (item count) |
| `prompt_editor` | string | — |

Shared options: `default`, `help`, `placeholder`, `optional` (unanswered → nil instead of
error), `answer_key` (answer under a different key), `cases` (case-variant expansion — see
`archetect learn cases`).

## Resolution order (same for every type)

1. An **answer** exists for the key (config → `-A` file → `-a` flag; last wins) → used,
   validated against the type.
2. Defaults apply (`--headless`, `-D`, or `-d <key>`) → the prompt's `default`; if `optional`
   and no default → nil.
3. Otherwise: ask interactively — or under `--headless`, ERROR
   `Headless mode: no answer or default for '<message>'`. Answer that key; re-run.

## The declarative mirror: `interface:`

The manifest (or sibling `interface.yaml`) can declare the same contract for external
tooling — web UIs, MCP clients, docs — without running the script:

```yaml
interface:
  prompts:
    - key: service_name
      type: text            # text|int|bool|select|multiselect|list|editor
      label: "Service Name:"
      required: true
      validation: "^[a-z][a-z0-9-]*$"
    - key: persistence
      type: select
      options: [Postgres, None]     # or {value,label,help} objects
  switches:
    - key: ci
      help: "Wire GitHub Actions"
```

The runtime does not enforce interface/script agreement yet — keep them in step by hand, and
treat a drift as a bug in the archetype.

Go deeper: `archetect learn rendering` (answering from the CLI) · `archetect learn mcp`
(prompts as a turn-based session).
