# Archetect 3: Model-Driven Architecture Generation

## Status snapshot (2026-04-17)

| Phase | Status |
|---|---|
| Phase 1 — Model schema + Lua library | in-progress (parser, types, builder in `archetect-aml/`; `archetect.model` Lua module registered; DAG resolution/expansion pending) |
| Phase 2 — Model-aware service archetypes | planned |
| Phase 3 — Cross-service wiring (client stubs, gateway, orchestrator) | planned |
| Phase 4 — Builder integration (full-system generation) | planned |
| Phase 5 — Multi-language type mappings | planned |

See also `docs/specs/architecture-spec-language.md` for the canonical AML spec.

## Vision

Generate entire architectures — services, gateways, orchestrators, adapters, persistence, client wiring, proto definitions — from a declarative domain model. A single YAML file describes the entities, their relationships, service boundaries, and the service dependency DAG. Archetect turns it into a complete, buildable system.

This is a v3-only, Lua-native feature. No v2 compatibility constraints.

## Design Principles

1. **Opt-in complexity.** Generating a plain boilerplate service requires zero model knowledge. The model is an enhancement, not a requirement.
2. **Declarative over imperative.** The model describes *what*, not *how*. Code generation patterns live in the archetypes.
3. **Relationships are first-class.** Not just entities and fields — the connections between them drive real code: foreign keys, client stubs, GraphQL stitching, proto imports.
4. **Service boundaries are explicit.** Each entity has a clear owner. Cross-service references generate client calls, not local joins.
5. **The DAG is the architecture.** Service-to-service dependencies drive orchestrator wiring, gateway integration, and client generation.
6. **Progressive detail.** A model can be as simple as entity names (get default CRUD) or as detailed as field-level validation rules, custom types, and relationship cardinalities.

## Architecture Model Schema

### Minimal Example

```yaml
# architecture.yaml
organization: acme
solution: commerce

domain:
  Customer:
    fields:
      name: String
      email: String

services:
  customer-service:
    owns: [Customer]
```

This is enough to generate a complete customer service with CRUD, persistence, proto definitions, and test scaffolding. Everything else is inferred from defaults.

### Full Example

```yaml
organization: acme
solution: commerce

types:
  Money:
    base: Decimal
    precision: 2
  
  OrderStatus:
    enum: [Draft, Submitted, Confirmed, Shipped, Delivered, Cancelled]

domain:
  Customer:
    fields:
      id: { type: UUID, key: true }
      name: { type: String, required: true }
      email: { type: String, required: true, unique: true }
      created_at: { type: Timestamp, auto: true }
  
  Product:
    fields:
      id: { type: UUID, key: true }
      name: { type: String, required: true }
      sku: { type: String, required: true, unique: true }
      price: { type: Money, required: true }
  
  Order:
    fields:
      id: { type: UUID, key: true }
      customer: { type: Customer, relation: many-to-one, required: true }
      status: { type: OrderStatus, default: Draft }
      total: { type: Money }
      created_at: { type: Timestamp, auto: true }
  
  LineItem:
    fields:
      id: { type: UUID, key: true }
      order: { type: Order, relation: many-to-one, required: true }
      product: { type: Product, relation: many-to-one, required: true }
      quantity: { type: Integer, required: true, min: 1 }
      price: { type: Money, required: true }

services:
  customer-service:
    type: service
    owns: [Customer]
    persistence: CockroachDB
  
  catalog-service:
    type: service
    owns: [Product]
    persistence: CockroachDB
  
  order-service:
    type: service
    owns: [Order, LineItem]
    persistence: CockroachDB
    calls: [customer-service, catalog-service]
  
  checkout-orchestrator:
    type: orchestrator
    calls: [order-service, customer-service, payment-adapter]
  
  payment-adapter:
    type: adapter
    calls: [order-service]
  
  commerce-gateway:
    type: domain-gateway
    exposes: [customer-service, catalog-service, order-service]

infrastructure:
  federated-gateway: true
  documentation: true
  platform-libs: true
```

### Schema Design Notes

**Field shorthand.** `name: String` expands to `name: { type: String }`. This keeps simple models concise.

**Implicit fields.** If no `key: true` field exists, an `id: UUID` primary key is generated. If no timestamp fields exist, `created_at` and `updated_at` are added. This is configurable via a `defaults` block.

**Relations.** When a field's type is another entity name, it's a relationship. The `relation` key specifies cardinality (`many-to-one`, `one-to-many`, `many-to-many`). If omitted, `many-to-one` is the default (foreign key on this entity).

**Cross-service references.** When Order references Customer but they're in different services, the generated code uses the customer-service client instead of a local join. The relation is still declared the same way — the service boundary determines the implementation.

**Custom types.** The `types` block defines domain-specific types that map to language primitives. Each archetype's templates know how to map `Money` → `BigDecimal` (Java) or `Decimal` (Rust).

## Lua Model Library

A `require("archetect.model")` module that archetypes use to work with the model:

```lua
local model = require("archetect.model")

-- Load from a YAML file or from context answers
local arch = model.load("architecture.yaml")
-- or
local arch = model.from_context(context)

-- Query the model
local service = arch:service("order-service")
local entities = service:entities()           -- [Order, LineItem]
local dependencies = service:dependencies()   -- [customer-service, catalog-service]
local callers = service:callers()             -- [checkout-orchestrator]

-- Entity details
local order = arch:entity("Order")
local fields = order:fields()                 -- all fields with full metadata
local relations = order:relations()           -- only relationship fields
local local_fields = order:local_fields()     -- non-relationship fields
local owned_by = order:service()              -- "order-service"

-- Cross-service awareness
local remote_refs = service:remote_references()  -- entities referenced but owned elsewhere
-- Returns: { Customer = "customer-service", Product = "catalog-service" }

-- Expand with case conventions for template use
local entity_ctx = model.expand(order)
-- Returns a table with all case variants for the entity name and every field name

-- Get the full DAG
local dag = arch:dag()
-- Returns adjacency list: { "checkout-orchestrator" = {"order-service", "customer-service", ...} }
```

### Implementation Options

**Option A: Built into archetect-core.** The model library is a Rust-backed Lua module, like `archetect.git` or `archetect.github`. This gives the best performance and the richest API. The model parsing, validation, and querying all happen in Rust.

**Option B: Pure Lua library.** Ship as a `.lua` file that archetypes can `require()`. Uses `format.from_yaml()` for parsing, pure Lua for querying. More portable, easier to iterate on, but less powerful for complex graph operations.

**Option C: Hybrid.** Core model parsing and DAG resolution in Rust (it's graph traversal, Rust is natural). Convenience query functions in Lua. Ship the Lua layer as a standard library archetype.

**Recommendation: Option A** for the core (load, parse, validate, DAG), with Lua convenience wrappers. The model is foundational infrastructure — it should be fast and correct. Lua scripts shouldn't need to hand-roll graph traversal.

## How Archetypes Use the Model

### Level 0: No Model (Backward Compatible)

Archetypes work exactly as they do today. Prompts, answers, context, render.

```lua
local context = Context.new()
context:prompt_text("Project Prefix:", "project_prefix", { ... })
-- ... normal flow
```

### Level 1: Default Model

Archetypes accept an optional model. If none provided, generate a default (current behavior):

```lua
local model = require("archetect.model")

-- Check if a model was provided via answers
local arch = model.from_context(context)
local service_def = arch and arch:service(context:get("project-name"))

if service_def then
    -- Model-driven: generate from the service definition
    local entities = service_def:entities()
    for _, entity in ipairs(entities) do
        local entity_ctx = model.expand(entity)
        context:set("entity", entity_ctx)
        directory.render("contents/entity", context)
    end
else
    -- No model: generate default entity from project prefix (current behavior)
    context:set("entity", default_entity(context:get("project-prefix")))
    directory.render("contents/entity", context)
end
```

### Level 2: Full Model-Driven Generation

The builder archetype loads the architecture model and distributes service-specific slices to each child:

```lua
local model = require("archetect.model")

-- Load the architecture model
local arch = model.load("architecture.yaml")
-- Or receive it from prompts/answers:
-- local arch = model.from_context(context)

-- Generate each service
for _, service in ipairs(arch:services()) do
    local service_type = service:type()  -- "service", "orchestrator", "adapter", etc.
    local archetype_name = type_to_archetype[service_type]
    
    -- Build the service-specific model slice
    context:set("service_model", service:to_context())
    -- Includes: owned entities, remote references, dependencies, caller info
    
    context:set("project-prefix", service:prefix())
    context:set("project-suffix", service:suffix())
    
    log.info("Rendering " .. service:name())
    component.render(archetype_name, context, {
        destination = context:get("organization-name"),
    })
end
```

### Level 3: AI-Assisted Model Construction

Via the MCP server, an AI agent can:

1. **Design the model** from natural language: "I need an e-commerce platform with customers, products, orders, and payments via Stripe"
2. **Generate the architecture YAML**
3. **Feed it to the builder** via `mcp_call(tool="render", arguments={source="builder", answers={model=...}})`
4. **Validate the output** compiles and tests pass
5. **Iterate** — "Add a reviews service that references products and customers"

## What Changes in Existing Archetypes

### Service Archetypes (Leaf)

Each service archetype gains an **optional** model-aware code path:

```lua
-- Check for model-provided entities
local service_model = context:get("service_model")
if service_model and service_model.entities then
    -- Model-driven: iterate real entities with real fields
    for _, entity in ipairs(service_model.entities) do
        -- entity has: name, fields (with types, relations, constraints), case variants
        context:set("entity", entity)
        directory.render("contents/entity", context)
        
        if entity.has_persistence then
            directory.render("contents/persistence_entity", context)
        end
    end
    
    -- Generate client stubs for remote dependencies
    for _, dep in ipairs(service_model.remote_refs) do
        context:set("remote_service", dep)
        directory.render("contents/client_integration", context)
    end
else
    -- Fallback: default entity from project prefix
    -- (existing behavior, unchanged)
end
```

### Builder Archetypes

The builder becomes simpler — it loads the model and delegates:

```lua
local model = require("archetect.model")
local arch = model.load_from_answers(context)

-- No more manual lists of gateways/services/adapters
-- The model declares them all
for _, service in ipairs(arch:services()) do
    context:set("service_model", service:to_context())
    component.render(service:archetype(), context, {
        destination = arch:org_solution_name(),
    })
end
```

### Template Enhancements

Templates gain richer entity data to work with:

```java
// Before (default model): just id and name
{% for field in entity.fields %}
    private {{ field.java_type }} {{ field.camelName }};
{% endfor %}

// After (full model): real fields with types, constraints, relations
{% for field in entity.local_fields %}
    {% if field.required %}@NotNull{% endif %}
    {% if field.unique %}@Column(unique = true){% endif %}
    private {{ field.java_type }} {{ field.camelName }};
{% endfor %}

{% for rel in entity.relations %}
    @{{ rel.jpa_annotation }}
    private {{ rel.java_type }} {{ rel.camelName }};
{% endfor %}
```

## Type Mapping

Each language ecosystem needs a type mapping. This can live in the archetype or in a shared library:

| Model Type | Java | Rust | Python | .NET |
|-----------|------|------|--------|------|
| String | String | String | str | string |
| Integer | Long | i64 | int | long |
| Decimal | BigDecimal | rust_decimal::Decimal | Decimal | decimal |
| UUID | UUID | Uuid | uuid.UUID | Guid |
| Boolean | Boolean | bool | bool | bool |
| Timestamp | Instant | DateTime<Utc> | datetime | DateTimeOffset |
| Date | LocalDate | NaiveDate | date | DateOnly |
| Enum(values) | enum | enum | Enum | enum |

Custom types (like `Money`) map through their `base` type.

## Relationship Mapping

| Relation | Same Service | Cross-Service |
|----------|-------------|---------------|
| many-to-one | JPA @ManyToOne + FK column | Client call + ID field |
| one-to-many | JPA @OneToMany | Not generated (query via client) |
| many-to-many | JPA @ManyToMany + join table | Not generated (use orchestrator) |

Cross-service relationships are the key insight: they generate **client integrations** instead of database joins.

## Implementation Phases

### Phase 1: Model Schema + Lua Library
- Define the YAML schema for architecture models
- Implement `archetect.model` Rust module (load, parse, validate, expand)
- Write tests with sample models

### Phase 2: Model-Aware Service Archetypes
- Add optional model code path to java-spring-boot-grpc-service
- Generate real entities, proto messages, persistence from model
- Prove the concept with one language ecosystem

### Phase 3: Cross-Service Wiring
- Generate client stubs from dependency graph
- Wire up gateways with entity-aware GraphQL schemas
- Generate orchestrator skeletons with typed client calls

### Phase 4: Builder Integration
- Architecture builder loads model and delegates to service archetypes
- Single command generates the full system
- MCP integration for AI-assisted model construction

### Phase 5: Multi-Language
- Extend to Rust, Python, .NET service archetypes
- Shared type mapping library
- Language-specific template enhancements

## Open Questions

1. **Where does the model file live?** Passed as an answer? A file path argument? Inline YAML in the builder prompt?
2. **Model validation.** How strict? Warn on missing relations? Error on circular dependencies?
3. **Incremental generation.** Can you re-run with an updated model and only regenerate changed services? (Probably a v4 concern.)
4. **Model inheritance/composition.** Can models import other models? ("Use the base customer model from the shared library.")
5. **Event-driven patterns.** The current model is request/response (service calls service). Should events/messages be a first-class concept?
