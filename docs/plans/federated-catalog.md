# Federated Catalog — Remote Server Entries

## Status snapshot (2026-04-18)

Phases 0–5 shipped end-to-end. A local catalog can declare a `server:`
entry; `archetect ls` / `archetect search` surface the remote tree with
a satellite marker; `archetect <acme/services/grpc>` routes the render
over gRPC to that server transparently.

| Phase | Scope | Status |
|---|---|---|
| 0 — Path-aware Initialize | `catalog_path` on Initialize; server resolves source by path, not just "default" | shipped |
| 1 — BrowseCatalog RPC | `BrowseCatalog(path?) → CatalogTree` and `SearchCatalog(query) → CatalogEntries` on the gRPC service | shipped |
| 2 — `server:` field on CatalogEntry | New manifest schema + mutual-exclusion validator (`source`/`catalog`/`server`) | shipped |
| 3 — Lazy remote indexer | `CatalogIndexer` fetches remote subtree via `BrowseCatalog`, splices as children with path rewriting; `RemoteEntryInfo` propagates down for render dispatch | shipped |
| 4 — Render dispatch | `PathTarget::Remote` routes via `client::start_remote`; per-entry TLS overlays top-level `client.tls` | shipped |
| 5 — `archetect ls` / `archetect search` | Federation roots get the 🛰️ icon; offline mode skips remote fetches gracefully | shipped |
| 6 — Security / trust | Opt-in mechanism; remote endpoints treated like shell-exec — `allowed` / `forbidden` / `prompt` | later |
| 7 — Auth | Deferred until the capability is in place and we have a server to dogfood against | later |

## Motivation

Orgs will accumulate private archetypes. Publishing them publicly on
GitHub isn't always possible (licensing, IP). Running an `archetect
server` exposing a private catalog works today — but it's a separate
workflow (`archetect connect --endpoint ...`) rather than a seamless
extension of the local catalog.

Federation closes that gap: a single top-level catalog entry points at
a remote `archetect server`, and `archetect ls` / `archetect search` /
`archetect render` treat the remote tree as if it were nested locally.

## Design

### Schema

```yaml
catalog:
  acme-internal:
    description: "Acme internal archetypes"
    server:
      endpoint: https://archetect.acme.corp:8443
      # Optional client TLS config; falls back to the top-level client.tls
      # section in archetect.yaml when omitted.
      tls:
        ca: /etc/archetect/acme-ca.crt
        domain: archetect.acme.corp
    show: true
```

`source:`, `catalog:`, and `server:` are mutually exclusive on a
single entry: an entry is either a git/local source, an inline
sub-catalog, or a remote pointer.

### gRPC protocol additions

```protobuf
service ArchetectService {
    // Existing
    rpc StreamingApi (stream ClientMessage) returns (stream ScriptMessage);
    // Phase 1
    rpc BrowseCatalog (BrowseCatalogRequest) returns (BrowseCatalogResponse);
    rpc SearchCatalog (SearchCatalogRequest) returns (SearchCatalogResponse);
}

message BrowseCatalogRequest {
    // Empty string = root.
    string path = 1;
}

message BrowseCatalogResponse {
    repeated CatalogIndexEntry entries = 1;
}

message CatalogIndexEntry {
    string path = 1;
    string name = 2;
    string description = 3;
    EntryKind kind = 4;
    bool is_archetype = 5;
    bool has_source = 6;
    bool show = 7;
    repeated CatalogIndexEntry children = 8;  // populated eagerly for small trees
}

enum EntryKind { GROUP = 0; LEAF = 1; }

message SearchCatalogRequest {
    string query = 1;
    bool include_hidden = 2;
}

message SearchCatalogResponse {
    repeated CatalogIndexEntry results = 1;
}
```

### Initialize extension (Phase 0)

```protobuf
message Initialize {
    string answers_yaml = 1;
    repeated string switches = 2;
    repeated string use_defaults = 3;
    bool use_defaults_all = 4;
    string destination = 5;
    // Phase 0: slash-separated catalog path. Empty = first/default entry
    // (existing behavior). When set, the server renders the archetype
    // at that catalog path.
    string catalog_path = 6;
}
```

Phase 0 is the keystone: once the server can render by catalog path,
the client (local federation) can just call `streaming_api` with the
remote path and the existing render pipeline handles everything.

### Client-side resolution flow

When a user navigates to `acme-internal/services/grpc`:

1. Local indexer walks the tree. At `acme-internal`, it finds a
   `server:` entry with no expanded children yet.
2. On demand (browse, search, or render dispatch), the indexer opens
   a gRPC client to the endpoint and calls `BrowseCatalog(path="")`.
3. The response tree is cached for the session and spliced in as the
   children of `acme-internal` with paths rewritten to include the
   `acme-internal/` prefix.
4. Render of a leaf like `acme-internal/services/grpc`: the client
   strips the `acme-internal/` prefix, opens a `streaming_api` stream
   to the endpoint, and sends Initialize with `catalog_path =
   "services/grpc"`. Prompts and file writes flow back through the
   normal IO channel.

### Caching

Session-scoped. A remote subtree is fetched once per CLI invocation.
For long-running processes (MCP, server) the cache TTL is 5 minutes.
No persistent cache in v1 — keep the blast radius of a misconfigured
remote small.

### Security (deferred)

Two knobs, modeled on the existing shell-exec policy:

```yaml
security:
  remote_server_policy: prompt    # allowed | forbidden | prompt
```

- `allowed` — connect without prompting (intended for org-internal
  deployments where trust is established at deploy time)
- `forbidden` — reject server entries entirely (useful for CI or
  untrusted environments)
- `prompt` — confirm once per session per endpoint

Auth (tokens, mTLS, OIDC) is orthogonal and ships after the capability
is in place. Until then, `security.remote_server_policy: forbidden` is
a reasonable default for shared configs.

## Non-goals

- No distributed catalog merging (e.g., two catalogs both claim the
  same path). The local entry name is authoritative.
- No remote-resource types beyond "archetect server". Git already
  handles remote archetypes; federation is only needed when the remote
  itself wants to mediate access.
- No persistent caching / pre-indexing in v1.

## Open questions

- Should `BrowseCatalog` return the full tree eagerly or one level at
  a time? Full tree is simpler to wire up; one-level-at-a-time scales
  better for very large remote catalogs. Start eager, revisit if it
  bites.
- Should `SearchCatalog` run server-side or just federate the entries
  into the local search index? Server-side is cheaper for large remote
  catalogs, local is simpler. Start local.
- How does this interact with the MCP catalog tools? They already do
  local browse/search. After federation, they transparently walk
  remote subtrees too — same code path.

## Exit criteria for each phase

- **Phase 0**: `archetect connect <url>` with `--` path form (or a
  new flag) renders a specific catalog entry on the remote server.
- **Phase 1**: a gRPC client can call `BrowseCatalog(path)` and get
  back the same tree structure `archetect ls` produces locally.
- **Phase 2**: `server:` in catalog.yaml parses and serializes
  without panicking; validator errors if `server:` and `source:`
  are both set.
- **Phase 3**: `archetect ls` shows the remote subtree under its
  federated entry name, paths correctly prefixed.
- **Phase 4**: `archetect render acme-internal/services/grpc`
  renders end-to-end with prompts and file writes.
- **Phase 5**: `archetect search` returns hits from remote catalogs.
- **Phase 6**: the `remote_server_policy` gate works in all three
  modes.
