# mcp — driving archetect as an MCP server

`archetect mcp` serves stdio MCP. The server resolves configuration ONCE at startup (catalog
index included); every render supplies an explicit `destination`. Shell-exec is FORBIDDEN in
MCP mode by design — a render needing `--allow-exec` is a CLI move.

| Tool | Mirrors | Notes |
|---|---|---|
| `learn { topic? }` / `introspect { filter? }` | `archetect learn` / `introspect` | the knowledge surface; topics also served as resources (`archetect://learn/<topic>`, `archetect://skill`) |
| `catalog_browse { path? }` / `catalog_search { query }` | `ls` / `search` | read-only, from the startup index |
| `render { source, destination, answers?, switches?, use_defaults_all? }` | `archetect render` | starts a stateful session; returns `complete`, `error`, or `prompting` + a PromptEnvelope |
| `catalog_render { path, … }` | bare `archetect <path>` | same session flow, source resolved from the catalog |
| `respond { value }` / `cancel {}` | (the terminal, inline) | answer the pending prompt / abort the session |

## The session loop

One render session at a time. Supply everything you know up front — `answers` as an object,
`switches` as a list (they are NOT prompted, ever) — and `use_defaults_all: true` when the
archetype's defaults are acceptable:

```
render { source, destination, answers = { service_name = "orders" }, use_defaults_all = true }
→ { status = "complete", files_written = [...] }        -- the goal: zero prompts
→ { status = "prompting", prompt = { type, key, message, options?, default?, … } }
   respond { value = "Postgres" }                        -- typed per prompt.type; null skips optional
   … repeat until complete/error
```

Prefer answering over responding: a `prompting` status means you lacked an answer — note the
key, and next time pass it in `answers`. The PromptEnvelope carries everything the archetype
declared (help, defaults, constraints, options).

## Shell out to the CLI for

`archetect eval '<lua>'` (the probe verb) · `--dry-run` / `--offline` / answer FILES /
per-key `-d` · `cache` verbs · `check` · `ide setup` · `config merged` · anything CI runs.
The MCP surface is for discovery and renders; the CLI is the full toolbox.

Go deeper: `archetect learn rendering` (the flags this mirrors) · `archetect learn prompts`
(what lands in a PromptEnvelope).
