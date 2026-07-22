# model — AML: generate architectures from a domain model

**Status first:** the AML foundation is SHIPPED — parser, typed model, query API, the
`require("archetect.model")` Lua module, and an interactive builder. The larger
model-driven-generation program (model-aware service archetypes, cross-service wiring,
full-system builders) is in progress — see `docs/plans/archetect-3-model-driven-generation.md`.
Write archetypes against the query API below; do not assume ecosystem archetypes consume
models yet.

A model is YAML describing entities, relationships, and service boundaries — the WHAT; the
archetypes own the HOW:

```yaml
# architecture.yaml
organization: acme
solution: commerce
domain:
  Customer: { fields: { name: string, email: string } }
  Order:
    fields: { total: decimal }
    relations: { customer: Customer }
boundaries:
  customer-service: { type: grpc, entities: [Customer] }
  order-service:    { type: grpc, entities: [Order] }
```

## Querying it from an archetype

```lua
local model = require("archetect.model")
local m = model.load("architecture.yaml")          -- or model.parse(yaml) / model.from_context(ctx)

for _, b in ipairs(m:all_boundaries()) do
  local ents = m:entities_for(b.name)              -- entities this service owns
  local deps = m:dependencies(b.name)              -- the service DAG edge list
  local remote = m:remote_references(b.name)       -- cross-boundary entity refs → client stubs
end
local slice = m:slice("order-service")             -- everything one service needs, in one shape
```

Query surface: `entity(name)` · `boundary(name)` · `all_boundaries()` ·
`boundaries_of_type(t)` · `entities_for(b)` · `outbound_interfaces(n)` /
`inbound_interfaces(n)` · `dependencies(n)` · `remote_references(n)` · `slice(n)` ·
`organization()` / `solution()` / `org_solution()`. Entities expand fields with case
variants pre-computed, so templates case-address them directly.

Building instead of loading: `model.builder()` (programmatic) or
`require("archetect.model.interactive").build(context)` — drives prompts to construct a
model when none exists.

## Decision rule

One service, flat answers → plain prompts are enough; skip the model. Several services
sharing entities, or anything with cross-service references → model first, then archetypes
query it. The model is opt-in complexity, never a prerequisite.

Shapes: `archetect introspect model`. Go deeper: `archetect learn composition` (rendering
per-boundary children).
