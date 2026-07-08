# Flag Resolution Semantics (Switches & Use-Defaults)

**Status:** Accepted — implementing for Archetect 3.0
**Scope:** `switches` and `use_defaults` resolution across all configuration
and orchestration layers; `use_defaults_all` symmetry; `name=false` negation
syntax.

## Problem

Switches and use-defaults grew organically, and each ingestion point
improvised its own merge behavior:

| Layer | Switches today | Use-defaults today | Answers today |
|---|---|---|---|
| Config files (user → etc.d → project → `--config-file`) | whole-array **replace** (config-crate artifact) | n/a | per-key overlay |
| CLI over config | additive **union** | additive (CLI only) | per-key overlay |
| Catalog entry over inherited context | whole-set **replace** | whole-set **replace** | per-key overlay |
| Lua child render opts over parent | whole-set **replace** | whole-set **replace** | per-key overlay |
| MCP render request | whole-set replace (caller owns set) | n/a | per-key overlay |

Consequences:

- A catalog entry declaring `switches: [with-health-check]` silently
  discards the user's `-s github`. Users type a flag and it vanishes.
- Once any layer enables a switch, there is **no opt-out** at a higher
  layer — additive-only union at the CLI, replace-to-clear everywhere
  else. Neither expresses "keep everything, but not this one."
- `use_defaults_all: true` on a catalog entry turns the flag on, but
  `use_defaults_all: false` is ignored — it can never be turned off.
- The rules cannot be documented in one sentence, which is how you know
  they were grown rather than designed.

## Design principle: documents vs flag bags

Every merged value is one of two kinds:

- **Document** — a coherent whole where partial merging produces
  nonsense. The catalog tree is a document: a project catalog *replaces*
  the global catalog. This stays as-is.
- **Flag bag** — a set of independent items, each meaningful alone.
  `switches`, `use_defaults`, and `answers` are flag bags. Flag bags
  merge by **per-item overlay**: each layer refines inherited state and
  never implicitly clears what it didn't mention.

Answers already follow the flag-bag rule at every layer. This spec
promotes `switches` and `use_defaults` to the same rule, and adds
negation syntax so overlay has an opt-out.

## Specification

### Token syntax

A switch or use-defaults token is one of:

```
name          # enable (sugar for name=true)
name=true     # enable
name=false    # disable (remove from the inherited set)
```

Any other `name=value` form is an **error**, reported with the offending
token and source. Names must be non-empty. This applies uniformly to:

- CLI: `-s github`, `-s github=false`, `--use-default port=false`
- Config files: `switches: [github, docker=false]`
- Catalog entries: `switches: [postgres=false]`, `use_defaults: [...]`
- Lua child render opts: `switches = { "github=false" }`
- MCP render request `switches` array

### Resolution pipeline

The final set is computed by folding layers **in precedence order**,
lowest to highest:

1. Built-in defaults
2. User config (`archetect.yaml` in the system config dir)
3. `etc.d/*.yaml` drop-ins (lexically sorted)
4. Project config (`.archetect.yaml` / `archetect.yaml` in CWD)
5. `--config-file`
6. CLI flags (`-s`, `--use-default`)
7. Parent archetype's child-render options (Lua `catalog.render(..., { switches = ... })`)
8. Catalog entry pre-configuration (applied at each rendered leaf)

Child renders inherit the parent's resolved flag bags (previously they
started empty). The leaf catalog entry is the last layer applied — it
is the most specific configuration for that render.

Each layer is an **overlay** on the accumulated set:

- `name` / `name=true` inserts `name`
- `name=false` removes `name`
- Items not mentioned are untouched — no layer implicitly clears
  inherited state.

Within a single layer, all additions apply before all removals, so
ordering inside one list never matters and `[x, x=false]` in one layer
deterministically resolves to "removed".

Scripts are unaffected: after resolution the context holds a plain set
of enabled names, and the script API (`archetype.switches.is_enabled`,
interface declarations) is unchanged. Negation is merge-time syntax
only; `=false` tokens never reach scripts.

### Config-file layering

The `config` crate replaces arrays wholesale between sources, which
made config-layer switch merging accidental "last file wins". The
`switches` field is now extracted per source and folded per the
pipeline above (same hand-extraction approach already used for the
project catalog). Other array fields are out of scope.

### `use_defaults_all`

Becomes a symmetric boolean overlay: a layer may set it `true` **or**
`false`; later layers win. (Previously only `true` was honored from
catalog entries.)

### Unchanged / out of scope

- **Answers** — already per-key overlay everywhere.
- **Catalog** — a document; project catalog replaces global (by design).
- **Script-visible API** — no changes to Lua/Rhai surfaces.
- **Interface manifests** — archetypes still declare available switches
  as plain names; negation is not meaningful in a declaration.

## Backwards compatibility

Syntax surfaces (`archetype.yaml`, `archetect.yaml`, CLI) are strictly
extended — every existing file parses and means what it meant, except
where behavior was replace-based:

- Catalog entries / Lua child opts that declare `switches` or
  `use_defaults` now **add to** the inherited set instead of replacing
  it. An entry that genuinely needs to suppress an inherited flag says
  so explicitly with `name=false`. (The old model could not express the
  new semantics; the new model expresses the old with explicit
  negation — the asymmetry that justifies the change.)
- Config files that declare `switches` now fold per-item across layers
  instead of last-file-wins.

This behavior change ships in 3.0, before public release, and is called
out in release notes. Production catalogs are audited/adjusted as
needed (tracked separately).

## One-sentence documentation target

> Later layers overlay earlier ones; switches and use-defaults merge
> per item; `name=false` removes an inherited item; nothing is cleared
> implicitly.
