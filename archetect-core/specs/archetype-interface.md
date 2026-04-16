# Archetype Interface Specification

> **Status:** Draft
> **Version:** 0.1.0

## Overview

An archetype's **interface** is an optional, declarative description of the
archetype's public input contract — what prompts it asks, what switches it
honours, and how clients should interact with it.

The interface can live in either of two places:

1. **`interface.yaml`** (preferred) — a standalone file alongside `archetype.yaml`.
2. **Inline `interface:` key** in `archetype.yaml` — convenient for archetypes
   with only a few prompts.

When both exist, the external file takes precedence.

The interface does **not** replace the Lua scripting engine. Lua remains the
execution engine for all archetype rendering. The interface is **metadata**
consumed by external tooling:

- **Web portals** — dynamically generate input forms instead of hand-crafting
  per-archetype UIs.
- **MCP / LLM agents** — discover required inputs so they can supply answers
  up front or choose interactive mode.
- **Documentation generators** — produce human-readable input references.
- **Validation tooling** — lint the interface against the Lua script to detect
  drift.

## Design Principles

1. **Lua drives execution.** The interface never changes rendering behaviour.
   It is purely descriptive.
2. **Minimal duplication.** Only declare what external consumers need —
   prompt type, key, label, constraints, options. Implementation details
   (case transforms, conditional branching, page sizes) stay in Lua.
3. **Progressive disclosure.** The section is optional. Archetypes without
   an `interface:` continue to work exactly as before.
4. **Switches become visible.** Today switches are invisible unless you read
   source. The interface makes them discoverable.
5. **Serializable.** The YAML schema maps trivially to JSON for web
   consumption.

## File Locations

### External file (preferred)

Place an `interface.yaml` (or `interface.yml`) alongside the manifest:

```
my-archetype/
├── archetype.yaml      # manifest — identity, catalog, templating
├── interface.yaml      # interface — prompts, switches, groups
├── archetype.lua       # script
└── contents/
```

The file contains the interface directly at the top level (no wrapping
`interface:` key):

```yaml
mode: batch
prompts:
  - key: project_name
    type: text
    label: "Project Name"
switches:
  - key: with_ci
    label: "Include CI"
```

### Inline (for simple archetypes)

For archetypes with only a few prompts, the interface can be inlined in
`archetype.yaml` under the `interface:` key:

```yaml
description: "Simple CLI"
requires:
  archetect: "3.0.0"
interface:
  prompts:
    - key: name
      type: text
      label: "Name"
```

### Precedence

When both `interface.yaml` and an inline `interface:` section exist, the
external file wins. This lets authors move the interface out of the
manifest without a breaking change.

## Schema

```yaml
  # How clients should interact with this archetype (optional).
  #   interactive: (default) Use prompt-by-prompt interactive protocol.
  #                Switches must be decided up front before execution.
  #   batch:       All required inputs are declared. Clients can render
  #                a complete form and submit all answers + switches at once.
  # Most archetypes omit this — interactive is always safe.
  # mode: batch

  prompts:
    - key: project_name          # required — maps to the answer key
      type: text                 # required — see Prompt Types
      label: "Project Name"      # required — human-readable label
      help: "Used for directory name, package name, module name"
      placeholder: "my-project"
      required: true             # default: true
      default: null              # type-appropriate default value

    - key: database
      type: select
      label: "Database"
      options:                   # required for select / multiselect
        - value: postgres
          label: "PostgreSQL"
        - value: mysql
          label: "MySQL"
        - value: sqlite
          label: "SQLite"
      default: postgres

    - key: features
      type: multiselect
      label: "Optional Features"
      options:
        - value: auth
          label: "Authentication"
          help: "Adds OAuth2 / OIDC middleware"
        - value: metrics
          label: "Observability"
          help: "Prometheus endpoint and structured logging"
        - value: grpc
          label: "gRPC API"
      min: 0                     # minimum selections
      max: null                  # no upper bound

    - key: port
      type: int
      label: "Server Port"
      default: 8080
      min: 1024
      max: 65535

    - key: enable_telemetry
      type: bool
      label: "Enable Telemetry"
      default: true

    - key: authors
      type: list
      label: "Authors"
      help: "One entry per author"
      min: 1

    - key: license_header
      type: editor
      label: "License Header"
      help: "Paste or edit your license header text"

  switches:
    - key: with_ci
      label: "Include CI/CD"
      help: "Generates GitHub Actions workflows"
      default: false

    - key: enterprise
      label: "Enterprise Mode"
      help: "Adds SSO, audit logging, and compliance scaffolding"
      default: false
```

## Prompt Types

Each prompt type maps to an existing `ScriptMessage::PromptFor*` variant.
The interface deliberately exposes a **subset** of the full prompt info
fields — only those relevant to external form generation.

### `text`

Free-form text input.

| Field         | Type              | Required | Default | Notes |
|---------------|-------------------|----------|---------|-------|
| `key`         | string            | yes      |         | Answer key |
| `label`       | string            | yes      |         | Display label |
| `help`        | string            | no       | null    | Help / description |
| `placeholder` | string            | no       | null    | Placeholder hint |
| `default`     | string            | no       | null    | Default value |
| `required`    | bool              | no       | true    | Whether input is required |
| `min`         | integer           | no       | null    | Minimum character length |
| `max`         | integer           | no       | null    | Maximum character length |
| `validation`  | string            | no       | null    | Regex pattern for validation |

### `int`

Integer input.

| Field         | Type              | Required | Default | Notes |
|---------------|-------------------|----------|---------|-------|
| `key`         | string            | yes      |         | Answer key |
| `label`       | string            | yes      |         | Display label |
| `help`        | string            | no       | null    | |
| `placeholder` | string            | no       | null    | |
| `default`     | integer           | no       | null    | |
| `required`    | bool              | no       | true    | |
| `min`         | integer           | no       | null    | Minimum value |
| `max`         | integer           | no       | null    | Maximum value |

### `bool`

Boolean toggle.

| Field         | Type              | Required | Default | Notes |
|---------------|-------------------|----------|---------|-------|
| `key`         | string            | yes      |         | Answer key |
| `label`       | string            | yes      |         | Display label |
| `help`        | string            | no       | null    | |
| `default`     | bool              | no       | null    | |
| `required`    | bool              | no       | true    | |

### `select`

Single selection from a list of options.

| Field         | Type              | Required | Default | Notes |
|---------------|-------------------|----------|---------|-------|
| `key`         | string            | yes      |         | Answer key |
| `label`       | string            | yes      |         | Display label |
| `help`        | string            | no       | null    | |
| `options`     | list              | yes      |         | See [Option Format](#option-format) |
| `default`     | string            | no       | null    | Must match an option `value` |
| `required`    | bool              | no       | true    | |

### `multiselect`

Multiple selection from a list of options.

| Field         | Type              | Required | Default | Notes |
|---------------|-------------------|----------|---------|-------|
| `key`         | string            | yes      |         | Answer key |
| `label`       | string            | yes      |         | Display label |
| `help`        | string            | no       | null    | |
| `options`     | list              | yes      |         | See [Option Format](#option-format) |
| `defaults`    | list of strings   | no       | []      | Pre-selected option `value`s |
| `required`    | bool              | no       | true    | |
| `min`         | integer           | no       | null    | Minimum selections |
| `max`         | integer           | no       | null    | Maximum selections |

### `list`

Freeform list of strings (user supplies entries, not chosen from options).

| Field         | Type              | Required | Default | Notes |
|---------------|-------------------|----------|---------|-------|
| `key`         | string            | yes      |         | Answer key |
| `label`       | string            | yes      |         | Display label |
| `help`        | string            | no       | null    | |
| `placeholder` | string            | no       | null    | Per-entry placeholder |
| `defaults`    | list of strings   | no       | []      | Default entries |
| `required`    | bool              | no       | true    | |
| `min`         | integer           | no       | null    | Minimum items |
| `max`         | integer           | no       | null    | Maximum items |

### `editor`

Multi-line text input (opens editor in TUI, renders as textarea in web).

| Field         | Type              | Required | Default | Notes |
|---------------|-------------------|----------|---------|-------|
| `key`         | string            | yes      |         | Answer key |
| `label`       | string            | yes      |         | Display label |
| `help`        | string            | no       | null    | |
| `placeholder` | string            | no       | null    | |
| `default`     | string            | no       | null    | |
| `required`    | bool              | no       | true    | |
| `min`         | integer           | no       | null    | Minimum character length |
| `max`         | integer           | no       | null    | Maximum character length |

## Option Format

Options for `select` and `multiselect` prompts accept two forms:

**Short form** — plain string (value and label are identical):

```yaml
options:
  - postgres
  - mysql
  - sqlite
```

**Long form** — object with value, label, and optional help:

```yaml
options:
  - value: postgres
    label: "PostgreSQL"
    help: "Recommended for production workloads"
  - value: mysql
    label: "MySQL"
  - value: sqlite
    label: "SQLite"
    help: "Embedded, no external dependencies"
```

Clients MUST support both forms. When short form is used, the string serves
as both `value` and `label`.

## Switches

Switches are boolean flags passed externally (CLI `--switch`, catalog entry
`switches:`, or API). They are **not prompted for** — they control hidden
behaviour branches in the Lua script.

| Field    | Type   | Required | Default | Notes |
|----------|--------|----------|---------|-------|
| `key`    | string | yes      |         | Switch name (passed to `archetype.switches.is_enabled()`) |
| `label`  | string | yes      |         | Human-readable label |
| `help`   | string | no       | null    | Description of what the switch enables |
| `default`| bool   | no       | false   | Default state |

## Interaction Modes

The `mode` field is optional — most archetypes omit it. It advises
clients on how to interact with the archetype:

| Mode          | Meaning |
|---------------|---------|
| `interactive` | **(default)** The archetype may have branching, conditional prompts, or dynamic behaviour. Clients should use the prompt-by-prompt interactive protocol (`ClientIoHandle`). The interface declares known inputs for discoverability, but the full set of prompts depends on runtime state. **Switches must be decided up front** — they are not prompted for during execution. |
| `batch`       | All required inputs are declared in the interface. Clients can render a complete form and submit all answers + switches at once. The Lua script will not ask for additional inputs beyond what is declared. |

**Why `interactive` is the default:** It is always safe. Even simple
archetypes work correctly in interactive mode — they just happen to have
a linear prompt flow. `batch` is an opt-in promise by the archetype
author that the interface is complete and the prompt flow is flat.

**Switches in either mode:** Switches are boolean flags that control
hidden behaviour. They are never prompted for — the Lua script reads
them via `archetype.switches.is_enabled(...)`. In interactive mode, the
client must set switches before starting the session. In batch mode,
switches are submitted alongside answers.

## Groups (Optional)

Prompts can be organized into labelled groups for UI layout purposes:

```yaml
groups:
    - label: "Project"
      keys: [project_name, description]
    - label: "Database"
      keys: [database, database_url]
    - label: "Features"
      keys: [features]
```

If `groups` is omitted, clients render prompts in declaration order as a
flat list. Keys not assigned to any group appear after all groups.

## JSON Representation

The interface is served to web clients and MCP agents as JSON. The mapping
is direct — YAML keys become JSON keys with no transformation:

```json
{
  "mode": "batch",
  "prompts": [
    {
      "key": "project_name",
      "type": "text",
      "label": "Project Name",
      "help": "Used for directory name, package name, module name",
      "placeholder": "my-project",
      "required": true,
      "default": null
    },
    {
      "key": "database",
      "type": "select",
      "label": "Database",
      "options": [
        { "value": "postgres", "label": "PostgreSQL" },
        { "value": "mysql", "label": "MySQL" },
        { "value": "sqlite", "label": "SQLite" }
      ],
      "default": "postgres",
      "required": true
    }
  ],
  "switches": [
    {
      "key": "with_ci",
      "label": "Include CI/CD",
      "help": "Generates GitHub Actions workflows",
      "default": false
    }
  ],
  "groups": null
}
```

## MCP Integration

The MCP server exposes the interface through existing and new tools:

### `archetype_interface`

Returns the parsed interface for a given archetype source.

```
archetype_interface { source: "https://github.com/org/my-archetype.git" }
→ { mode, prompts, switches, groups }
```

### Enhanced `catalog_search` / `catalog_browse`

Search and browse results include a summary of each archetype's interface
when available:

```
catalog_search { query: "rust cli" }
→ [
    {
      "path": "rust/cli/rust-clap-cli",
      "description": "Rust CLI with clap",
      "interface": {
        "mode": "batch",
        "prompt_count": 3,
        "switch_count": 1,
        "prompt_keys": ["project_name", "description", "features"]
      }
    }
  ]
```

The full interface is fetched on demand via `archetype_interface`.

## Web Client Workflow

### Batch mode

```
1. Client fetches interface JSON for the archetype
2. Client renders form controls from prompts + switches
3. User fills in form
4. Client validates required fields, min/max constraints
5. Client submits answers map + switches to archetect (headless mode)
6. Archetect runs Lua script with pre-supplied answers
7. If any prompt is missing from answers → error (headless rejects)
8. Output streamed back to client
```

### Interactive mode

```
1. Client optionally pre-fills known answers from interface
2. Client opens WebSocket / streaming connection to archetect
3. Archetect runs Lua script, sends ScriptMessage for each prompt
4. Client renders UI control dynamically for each prompt
5. User responds, client sends ClientMessage back
6. Loop until CompleteSuccess / CompleteError
7. Output streamed back to client
```

### Hybrid mode

```
1. Client renders form from interface (batch-style pre-fill)
2. User fills in what they can, clicks "Generate"
3. Client opens interactive session with pre-filled answers
4. Lua script auto-accepts pre-filled answers (existing behaviour)
5. For prompts not in the interface (conditional branches, etc.),
   the UI renders them inline as they arrive
6. Best of both: fast form UX for the common path, full fidelity
   for complex archetypes
```

## Validation and Drift Detection

A `archetect lint` command (future work) can compare the declared interface
against the Lua script's actual prompts:

- **Missing prompt:** Script calls `context:text("project_name", ...)` but
  the interface has no prompt with `key: project_name`. Warning: undeclared
  prompt.
- **Extra prompt:** Interface declares a prompt with `key: unused_key` but
  the script never prompts for it. Warning: declared but unused.
- **Type mismatch:** Interface declares `type: select` but script calls
  `context:text(...)` for the same key. Error.
- **Missing switch:** Script checks `archetype.switches.is_enabled("foo")`
  but the interface doesn't list it. Warning: undeclared switch.

These are warnings, not errors — the interface is advisory, and some
prompts are inherently conditional and may not appear in every execution
path.

## Backwards Compatibility

- Both the `interface.yaml` file and the inline `interface:` key are
  optional. Existing archetypes without either are unaffected.
- Parsing ignores unknown keys within the interface for forward compat.
- Clients that don't understand the interface (older archetect versions,
  plain CLI usage) simply ignore it.
- No changes to the Lua API, prompt protocol, or rendering behaviour.
