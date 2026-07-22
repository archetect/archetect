# catalogs — archetypes all the way down

A catalog is just a manifest whose `catalog:` map has entries (and no `archetype.lua`).
Groups nest arbitrarily; leaves point at renderable sources; federation entries splice in a
remote server's tree. One format, one mental model.

```yaml
# archetype.yaml — a catalog
catalog:
  services:
    description: Backend services
    catalog:                                  # group → submenu
      grpc:
        source: https://github.com/acme/grpc-service.git#v1     # leaf → archetype
        answers: { team: platform }           # pre-supplied, most-specific layer
        switches: [ci]
  partner:
    server: { endpoint: "https://archetect.partner.dev" }       # federation → remote subtree
  scm-lib:
    source: https://github.com/acme/scm-library.git#v1
    library: true                             # staged into consumers' require() path
    show: false                               # hidden from menus, addressable by name
```

## Walking and rendering

[[slot:catalog_tree]]

- `archetect ls [path]` (`-a` shows hidden/component entries) · `archetect search <terms>`
  (AND over name/description/path/tags/languages/frameworks).
- Render an entry by PATH: `archetect services/grpc` (the bare form dispatches into the
  configured catalog) — entry answers/switches overlay config, CLI flags overlay both.
- A group path prompts a menu interactively; in automation always name a LEAF.

## Entry kinds, exactly one per entry

| Kind | Key | Behavior |
|---|---|---|
| leaf | `source:` | resolve + render the archetype |
| group | `catalog:` | nested entries |
| federation | `server:` | children fetched from a remote archetect server on demand; renders route over gRPC; TLS per-entry or from `client.tls` |

Per-entry flags: `answers`, `switches`, `use_defaults`, `use_defaults_all` (the
most-specific overlay layer), `library` (eager-stage `lib/` + `includes/` for consumers —
`archetect learn composition`), `show: false` (hide from menus; scripts and paths still
reach it).

Sources accept git URLs (`#tag`/`#branch`/`#commit` refs), SSH shorthand, and local paths —
relative paths resolve against the CATALOG FILE's directory, not your cwd. Resolution and
caching: `archetect learn sources`.

Go deeper: `archetect learn composition` (scripts rendering entries) · `archetect learn
manifest` (the entry schema).
