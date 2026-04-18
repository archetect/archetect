# Archetect Modeling Language (AML)

## Status snapshot (2026-04-17)

| Component | Status |
|---|---|
| AML model (YAML schema — entities, boundaries, interfaces, flows, types) | in-progress (parser + types shipped in `archetect-aml/`; validation and DAG resolution pending) |
| `archetect.model` Lua query API (entities, relations, interfaces, dependencies) | in-progress |
| Orchestrator engine (dispatch, phases) | planned — no code yet |
| Archetype capabilities registry (`capabilities:` manifest block) | in-progress (schema drafted; convention-based resolution not yet wired) |
| Profiles (archetype selection for boundary properties) | planned |
| Model Slice Contract (per-boundary context preparation) | in-progress |
| Progressive adoption — Level 0 (direct render) | shipped |
| Progressive adoption — Level 1 (model-aware archetypes) | in-progress |
| Progressive adoption — Level 2 (orchestrated) | planned |
| AML as standalone artifact (UI, agents, other tools) | planned |

## Overview

A canonical modeling language for describing software architectures — entities, relationships, service boundaries, communication patterns, and data flows. The language is technology-agnostic: it describes **what** the system is, not **how** it's implemented.

The language supports:
- Single applications and multi-service architectures
- Synchronous and asynchronous communication
- In-process and cross-process boundaries
- Schema validation
- Consumption by UIs (drag-and-drop), AI agents, and CLI tools
- Co-authoring between humans and machines

## System Architecture

### Three Layers

```
┌─────────────────────────────────────────────────┐
│   AML Model (What)                              │
│   "Order-service communicates with              │
│    inventory-service asynchronously"             │
├─────────────────────────────────────────────────┤
│   Orchestrator (Algorithmic Dispatch)            │
│   Walks the model, selects archetypes,          │
│   feeds model slices to each                    │
├──────────────┬──────────────┬───────────────────┤
│  Archetype A │  Archetype B │  Archetype C ...  │
│  (Java gRPC  │  (Rust GW)   │  (Python async)   │
│   service)   │              │                    │
└──────────────┴──────────────┴───────────────────┘
```

**AML layer:** The model. Technology-agnostic. Describes entities, boundaries, interfaces, flows.

**Orchestrator layer:** NOT an archetype. A higher-level engine built into Archetect that algorithmically walks the model, resolves which archetype to use for each boundary (based on language, type, protocol, etc.), and invokes them with the appropriate model slice. Think of it like a build system that invokes compilers — the orchestrator is Make; the archetypes are gcc/rustc/javac.

**Archetype layer:** Pluggable implementation components. Each archetype knows how to generate one kind of thing (a Java gRPC service, a Rust gateway, a Python async consumer). They receive a model slice and produce code. They don't know about the full architecture — just their piece.

### Why the Orchestrator Is Not an Archetype

An archetype renders templates from a context. The orchestrator doesn't render anything. It:

1. Parses the AML model
2. Resolves the dependency DAG
3. For each boundary, determines which archetype to invoke based on model properties
4. Prepares the model slice (owned entities, interfaces, dependencies)
5. Invokes the archetype with the slice
6. Manages cross-cutting concerns (shared types, infrastructure)

Making this an archetype leads to the "uber builder" anti-pattern — one mega-archetype that hard-codes knowledge of every language, framework, and protocol. Instead, the orchestrator is generic and algorithmic. The archetypes are specific and pluggable.

### Archetype Resolution

The orchestrator needs to map model properties to archetypes. Two approaches, both supported:

**Convention-based resolution:**
```yaml
# The model says:
boundaries:
  order-service:
    language: java
    type: service
    protocol: grpc
    persistence: cockroachdb

# The orchestrator resolves:
#   language=java + type=service + protocol=grpc
#   → java-spring-boot-grpc-service.archetype3
```

The orchestrator maintains a registry of archetypes tagged by capabilities. Given a set of model properties, it finds the best-matching archetype. This is the "plug in only Rust archetypes and get an all-Rust architecture" scenario — you register Rust archetypes and the orchestrator selects them for every boundary.

**Explicit mapping:**
```yaml
# In the model or a companion file:
archetypes:
  java-service: git@github.com:p6m-archetypes/java-spring-boot-grpc-service.archetype3.git
  rust-gateway: git@github.com:p6m-archetypes/rust-graphql-federated-gateway.archetype3.git
  python-consumer: git@github.com:p6m-archetypes/python-async-consumer.archetype3.git

# Boundaries reference them:
boundaries:
  order-service:
    archetype: java-service
```

Both approaches can coexist. Explicit mapping overrides convention-based resolution.

### Archetype Capabilities Registry

For convention-based resolution, archetypes declare their capabilities in their manifest:

```yaml
# archetype.yaml for java-spring-boot-grpc-service
description: "Java Spring Boot gRPC Service"
requires:
  archetect: "3.0.0"

capabilities:
  language: java
  type: service
  protocols: [grpc]
  persistence: [cockroachdb, postgresql, none]
  features: [crud, events, client-generation]
```

The orchestrator queries: "Give me an archetype that handles `language=java, type=service, protocol=grpc, persistence=cockroachdb`" and gets a match.

This means:
- Registering your company's custom archetypes lets the orchestrator use them
- Swapping Java for Rust means registering Rust archetypes with the same capability tags
- New persistence backends just need an archetype that declares the capability
- The orchestrator algorithm stays the same

### The Five Primitives

1. **Entity** — A domain object with fields and constraints
2. **Boundary** — A deployment/process/ownership unit (a service, a module, a package)
3. **Interface** — How two boundaries communicate (sync, async, query, command)
4. **Flow** — A sequence of interactions across boundaries for a use case
5. **Type** — A reusable data type definition (enums, value objects, custom scalars)

## Spec Format

### Root Structure

```yaml
spec: "1.0"

# Identity
name: acme-commerce
organization: acme
solution: commerce
description: "E-commerce platform for Acme Corp"

# Reusable type definitions
types:
  Money: { ... }
  OrderStatus: { ... }

# Domain entities
entities:
  Customer: { ... }
  Order: { ... }

# Service/module boundaries
boundaries:
  customer-service: { ... }
  order-service: { ... }

# Communication patterns between boundaries
interfaces:
  - from: order-service
    to: customer-service
    style: sync
    
  - from: order-service
    to: notification-service
    style: async
    events: [OrderCreated, OrderShipped]

# Use-case flows (optional, for documentation and orchestrator generation)
flows:
  checkout:
    steps: [...]
```

### Types

Types define reusable data types beyond the built-in primitives.

```yaml
types:
  # Value type with underlying primitive
  Money:
    base: Decimal
    precision: 2
    description: "Monetary amount"
  
  # Enumeration
  OrderStatus:
    enum:
      - Draft
      - Submitted
      - Confirmed
      - Shipped
      - Delivered
      - Cancelled
    description: "Lifecycle states for an order"
  
  # Structured value object (embedded, not a separate entity)
  Address:
    value:
      street: String
      city: String
      state: String
      zip: String
      country: { type: String, default: "US" }
  
  # Collection/generic types
  PagedResult:
    generic: [T]
    value:
      items: { type: List, of: T }
      total: Integer
      page: Integer
      page_size: Integer
```

**Built-in primitives:** String, Integer, Long, Decimal, Boolean, UUID, Date, Timestamp, Bytes, List, Map

### Entities

Entities are the domain objects — the nouns of the system.

```yaml
entities:
  Customer:
    description: "A customer account"
    fields:
      id: UUID                          # Shorthand: just the type
      name: { type: String, required: true, min: 1, max: 200 }
      email: { type: String, required: true, unique: true, format: email }
      status: { type: CustomerStatus, default: Active }
      address: Address                  # Embedded value object
      created_at: { type: Timestamp, auto: create }
      updated_at: { type: Timestamp, auto: update }
    
  Order:
    description: "A customer order"
    fields:
      id: UUID
      customer: { entity: Customer, relation: many-to-one, required: true }
      status: { type: OrderStatus, default: Draft }
      items: { entity: LineItem, relation: one-to-many }
      total: Money
      created_at: { type: Timestamp, auto: create }
    operations:               # Optional: explicit CRUD + custom operations
      - create
      - read
      - update
      - list
      - submit               # Custom operation
      - cancel               # Custom operation
    events:                   # Events this entity can emit
      - OrderCreated
      - OrderSubmitted
      - OrderCancelled
  
  LineItem:
    description: "A line in an order"
    fields:
      id: UUID
      order: { entity: Order, relation: many-to-one, required: true }
      product: { entity: Product, relation: many-to-one, required: true }
      quantity: { type: Integer, required: true, min: 1 }
      unit_price: Money
```

**Field shorthand rules:**
- `name: String` expands to `{ type: String }`
- `name: UUID` when named `id` implies `{ type: UUID, key: true, auto: create }`
- `customer: { entity: Customer }` implies `{ entity: Customer, relation: many-to-one }` (FK default)

**Relationship types:**
- `many-to-one` — This entity holds a foreign key
- `one-to-many` — The other entity holds the FK (inverse side)
- `many-to-many` — Join table (or join entity for additional fields)
- `one-to-one` — Shared key or unique FK

### Boundaries

Boundaries define ownership and deployment units. A boundary can be a service, a module within a monolith, or a package within a library.

```yaml
boundaries:
  # A microservice
  customer-service:
    type: service
    owns: [Customer]
    persistence: true               # Needs a database (type decided by interpretation)
    description: "Manages customer accounts"
  
  # Another microservice
  order-service:
    type: service
    owns: [Order, LineItem]
    persistence: true
    description: "Manages orders and line items"
  
  # An orchestrator (no owned entities, coordinates workflows)
  checkout-orchestrator:
    type: orchestrator
    description: "Coordinates the checkout workflow"
  
  # An adapter (bridges to external systems)
  payment-adapter:
    type: adapter
    external: stripe
    description: "Integrates with Stripe for payments"
  
  # A gateway (exposes services to external clients)
  commerce-gateway:
    type: gateway
    style: graphql                  # The one place where protocol leaks in —
                                    # because the gateway IS the protocol choice
    description: "Public API for the commerce platform"
  
  # A shared library (not a service, but a code boundary)
  commerce-domain:
    type: library
    owns: [Money, OrderStatus, Address]
    description: "Shared domain types"
```

**Boundary types:**
- `service` — A deployable unit that owns entities and exposes operations
- `orchestrator` — Coordinates workflows across services (no owned entities)
- `adapter` — Bridges to external systems
- `gateway` — Entry point for external clients
- `library` — Shared code, not independently deployed
- `module` — A boundary within a monolith (same process, different ownership)

### Interfaces

Interfaces describe how boundaries communicate. This is where sync vs async, commands vs queries, and events vs requests are declared — without prescribing technology.

```yaml
interfaces:
  # Synchronous request/response
  - from: order-service
    to: customer-service
    style: sync
    operations:
      - GetCustomer
      - ValidateCustomer

  # Synchronous but could also be modeled as async
  - from: checkout-orchestrator
    to: order-service
    style: sync
    operations:
      - CreateOrder
      - SubmitOrder
  
  - from: checkout-orchestrator
    to: payment-adapter
    style: sync
    operations:
      - ProcessPayment
  
  # Asynchronous event-driven
  - from: order-service
    to: notification-service
    style: async
    events:
      - OrderCreated
      - OrderShipped
      - OrderCancelled
  
  # Asynchronous command (fire-and-forget)
  - from: checkout-orchestrator
    to: fulfillment-service
    style: async
    commands:
      - FulfillOrder
  
  # Gateway exposure
  - from: commerce-gateway
    to: customer-service
    style: sync
    expose: [Customer]              # Which entities to expose through the gateway
    
  - from: commerce-gateway
    to: order-service
    style: sync
    expose: [Order]

  # In-process (module boundaries within a monolith)
  - from: order-module
    to: customer-module
    style: in-process
```

**Interface styles:**
- `sync` — Request/response. Interpretation decides: gRPC, REST, HTTP, in-process call
- `async` — Fire-and-forget or pub/sub. Interpretation decides: SQS, Kafka, RabbitMQ, in-process channel
- `in-process` — Direct function/method call within the same deployment unit
- `stream` — Long-lived bidirectional communication. Interpretation decides: gRPC streaming, WebSocket, SSE

### Flows (Optional)

Flows describe use-case sequences. They're useful for generating orchestrator logic and for documentation (sequence diagrams).

```yaml
flows:
  checkout:
    description: "Customer completes a purchase"
    trigger: "Customer clicks 'Place Order'"
    steps:
      - service: customer-service
        operation: ValidateCustomer
        description: "Verify customer exists and is active"
      
      - service: order-service
        operation: CreateOrder
        with: { status: Draft }
      
      - service: payment-adapter
        operation: ProcessPayment
        on_failure: 
          - service: order-service
            operation: CancelOrder
      
      - service: order-service
        operation: SubmitOrder
        emits: OrderCreated            # Triggers async subscribers
      
      - service: fulfillment-service
        operation: FulfillOrder
        style: async                   # This step is fire-and-forget
```

Flows are optional but powerful: they can generate orchestrator boilerplate, integration test scenarios, and sequence diagram documentation.

## The Orchestrator

The orchestrator is a first-class Archetect concept — not an archetype, but the engine that drives archetype invocation from an AML model.

### How It Works

```
archetect generate architecture.yaml --profile p6m-java
```

1. **Parse** — Load and validate the AML model
2. **Resolve** — For each boundary, determine the archetype to use (from profile + capabilities registry)
3. **Plan** — Compute the generation order (respecting the DAG — shared libraries first, then services, then gateways)
4. **Slice** — For each boundary, extract its model slice: owned entities (with full field/relation detail), inbound interfaces, outbound interfaces, dependency list
5. **Dispatch** — Invoke each archetype with its slice. The archetype receives a well-defined model context and generates code.
6. **Cross-cut** — Generate cross-cutting concerns: infrastructure (from async interface declarations), shared type libraries, documentation

### Profiles

A profile bundles archetype selections and default configurations. This replaces the "interpretation" concept — it's more concrete and maps directly to Archetect's existing concepts.

```yaml
# profiles/p6m-java.yaml
name: p6m-java
description: "P6M standard Java stack"

# Archetype mappings by boundary type + properties
archetypes:
  service:
    default: p6m-archetypes/java-spring-boot-grpc-service.archetype3
    when:
      protocol: rest
      archetype: p6m-archetypes/java-spring-boot-rest-service.archetype3
  
  orchestrator:
    default: p6m-archetypes/java-spring-boot-grpc-service.archetype3
  
  adapter:
    default: p6m-archetypes/java-spring-boot-grpc-service.archetype3
  
  gateway:
    default: p6m-archetypes/java-spring-boot-graphql-domain-gateway.archetype3
    when:
      style: federated
      archetype: p6m-archetypes/rust-graphql-federated-gateway.archetype3
  
  library:
    default: p6m-archetypes/java-platform-libs.archetype3
  
  # Assessor is a custom boundary type — profiles are extensible
  assessor:
    default: p6m-archetypes/java-spring-boot-grpc-assessor.archetype3

defaults:
  persistence: cockroachdb
  async: sqs
  ci-cd: github-actions
  artifactory-host: p6m.jfrog.io
```

A polyglot profile:
```yaml
# profiles/p6m-polyglot.yaml
name: p6m-polyglot

archetypes:
  service:
    when:
      language: java
      archetype: p6m-archetypes/java-spring-boot-grpc-service.archetype3
    when:
      language: rust
      archetype: p6m-archetypes/rust-grpc-service-axum-modular.archetype3
    when:
      language: python
      archetype: p6m-archetypes/python-grpc-service-uv.archetype3
    when:
      language: dotnet
      archetype: p6m-archetypes/dotnet-grpc-service.archetype3
  
  gateway:
    default: p6m-archetypes/rust-graphql-federated-gateway.archetype3
```

Now the AML model can declare `language: rust` on a boundary and the profile selects the right archetype:

```yaml
boundaries:
  order-service:
    type: service
    language: rust              # ← Profile resolves to rust-grpc-service-axum-modular
    owns: [Order, LineItem]
```

### Model Slice Contract

Each archetype receives a well-defined model slice — not raw YAML, but a structured context prepared by the orchestrator:

```lua
-- What an archetype receives from the orchestrator:
local model = context:get("_model")

model.boundary          -- This boundary's definition
model.entities          -- Owned entities, fully expanded with case variants and field metadata
model.local_relations   -- Relations between owned entities (same service)
model.remote_relations  -- Relations to entities in other services (→ generate client calls)
model.inbound           -- Interfaces where other services call this one
model.outbound          -- Interfaces where this service calls others
model.events_published  -- Events this service emits
model.events_consumed   -- Events this service subscribes to
model.dependencies      -- Services this one depends on (for client generation)
model.dependents        -- Services that depend on this one
model.types             -- Custom types referenced by owned entities
```

The archetype doesn't parse AML. It doesn't walk the DAG. It receives a clean, pre-processed slice and generates code from it. This keeps archetypes simple and focused.

### Progressive Adoption

The orchestrator is a new capability, but it doesn't replace the existing archetype model:

**Level 0: Direct archetype render (existing)**
```
archetect render java-spring-boot-grpc-service.archetype3 ./output
```
Interactive prompts, no model. Works exactly as today.

**Level 1: Model-aware archetype (opt-in)**
```
archetect render java-spring-boot-grpc-service.archetype3 ./output -a model=service-model.yaml
```
The archetype detects a model in its answers and uses it instead of prompting for entity details. Fallback to prompts if no model.

**Level 2: Orchestrated generation (new)**
```
archetect generate architecture.yaml --profile p6m-java
```
The orchestrator reads the full AML model, slices it, and dispatches to archetypes. This is the new capability.

Level 0 and Level 1 don't require the orchestrator. Level 2 does. Archetypes that work at Level 1 automatically work at Level 2 — the orchestrator just provides the model slice that the archetype would otherwise get from answers.

## Single Application Support

The same spec works for a single application — just one boundary:

```yaml
spec: "1.0"
name: my-crud-app

entities:
  User:
    fields:
      id: UUID
      name: { type: String, required: true }
      email: { type: String, required: true, unique: true }
  
  Post:
    fields:
      id: UUID
      author: { entity: User, relation: many-to-one }
      title: { type: String, required: true }
      body: String
      published: { type: Boolean, default: false }

boundaries:
  my-app:
    type: service
    owns: [User, Post]
    persistence: true
```

No interfaces needed (single boundary). No flows needed. This generates a straightforward CRUD service with two entities and their relationship — exactly what you'd want for a quick prototype.

## Monolith Support

For modular monoliths, boundaries with `in-process` interfaces share a deployment unit:

```yaml
boundaries:
  user-module:
    type: module
    deployment: my-monolith
    owns: [User]
  
  post-module:
    type: module
    deployment: my-monolith
    owns: [Post]

interfaces:
  - from: post-module
    to: user-module
    style: in-process
```

This generates module boundaries within a single application — separate packages/namespaces, but no network calls. Later, splitting to microservices means changing `type: module` to `type: service` and `style: in-process` to `style: sync`.

## UI / Agent Authoring

The spec is designed to be authored through multiple channels:

### YAML (Developer)
Direct editing for power users. Schema validation catches errors.

### UI (Drag-and-Drop)
A visual editor where:
- Entities are boxes with field lists
- Boundaries are containers (drag entities into them)
- Interfaces are arrows between boundaries (select sync/async/style)
- Flows are sequence diagrams you can draw

The UI serializes to/from the same YAML spec.

### AI Agent
An AI agent can:
- Generate a spec from natural language: "I need an e-commerce platform with..."
- Modify an existing spec: "Add a reviews service that references products"
- Validate a spec against best practices
- Suggest missing interfaces or boundaries

All three channels produce the same spec format. The builder archetype doesn't care who authored it.

## Schema Validation

The spec has a JSON Schema / YAML Schema that enables:
- IDE autocompletion (YAML language server)
- Pre-generation validation (catch errors before rendering)
- UI constraint enforcement
- Agent output validation

Validation rules:
- Every entity referenced in a relationship must exist
- Every entity must be owned by exactly one boundary
- Interface endpoints must reference declared boundaries
- Enum types used in fields must be defined in `types`
- Circular dependencies in sync interfaces produce warnings
- Flow steps must reference declared operations or entity CRUD

## Integration with Archetect

### New Concepts in Archetect

AML introduces two new first-class concepts to Archetect:

1. **`archetect generate`** — A new top-level command (alongside `render`) that takes an AML file + profile and runs the orchestrator
2. **`capabilities`** — A new section in `archetype.yaml` that declares what an archetype can generate, enabling convention-based resolution
3. **Profiles** — Configuration files that map model properties to archetype selections

These are additive. Existing `archetect render` and all existing archetypes continue to work unchanged.

### What Gets Built in Rust (archetect-core)

- AML parser and validator (YAML → typed model structs)
- DAG resolver (topological sort of boundaries by dependency)
- Profile loader and archetype resolver
- Model slicer (extract per-boundary view with expanded entities, interfaces, dependencies)
- JSON Schema generation for AML validation

### What Gets Built in Lua (archetect.model)

- `model.from_context(context)` — Access the model slice passed by the orchestrator
- Entity query helpers (fields, relations, local vs remote)
- Case expansion for entity/field names
- Type mapping lookups (model type → language type)

### What Archetypes Need

Archetypes that want to participate in orchestrated generation need:
1. A `capabilities` block in `archetype.yaml` (for convention-based resolution)
2. An optional model-aware code path in their Lua script (check for `_model` in context)
3. Template enhancements for entity/field iteration

Archetypes without these continue to work for direct `archetect render` invocations.

### AML as a Standalone Artifact

The AML spec is consumed by Archetect but is not Archetect-specific. The YAML format and JSON Schema can be consumed by:
- UI editors (drag-and-drop architecture design)
- AI agents (generate/modify architecture specs)
- Documentation tools (generate architecture diagrams)
- Validation tools (check for anti-patterns, circular dependencies)
- Other code generators

The AML file can be:
- A file path: `archetect generate architecture.yaml --profile p6m-java`
- Passed via MCP: AI agent generates AML and feeds it to the orchestrator
- Stored in a repository: versioned alongside the generated code
- Composed from fragments: shared entity libraries + project-specific boundaries

## Example: From Spec to Generated System

Given the full commerce spec above with a Java/Spring Boot/gRPC/CockroachDB interpretation:

```
acme-commerce/
├── customer-service/
│   ├── customer-service-api/          # Generated proto + Java interfaces
│   │   └── src/.../api/v1/
│   │       ├── Customer.java          # Generated from entity fields
│   │       └── CustomerService.java   # CRUD operations
│   ├── customer-service-core/         # Business logic stubs
│   ├── customer-service-server/       # gRPC server impl
│   ├── customer-service-client/       # Client library for other services
│   ├── customer-service-grpc/         # Proto definitions
│   │   └── customer_v1.proto          # Generated from entity + operations
│   └── .platform/kubernetes/          # Kubernetes manifests
│
├── order-service/
│   ├── order-service-api/
│   │   └── src/.../api/v1/
│   │       ├── Order.java             # Full entity with customer FK
│   │       ├── LineItem.java          # With order FK + product ref (client)
│   │       └── OrderService.java      # CRUD + Submit + Cancel
│   ├── order-service-core/
│   │   └── src/.../core/
│   │       ├── CustomerClient.java    # ← Generated from interface declaration
│   │       └── OrderEventPublisher.java # ← Generated from entity events
│   ├── order-service-server/
│   └── order-service-grpc/
│       └── order_v1.proto             # Includes LineItem, references Product by ID
│
├── checkout-orchestrator/
│   └── src/.../orchestrator/
│       ├── CheckoutFlow.java          # ← Generated from flow definition
│       ├── OrderClient.java           # ← From interface: sync to order-service
│       ├── CustomerClient.java        # ← From interface: sync to customer-service
│       └── PaymentClient.java         # ← From interface: sync to payment-adapter
│
├── notification-service/              # Python (per interpretation override)
│   └── src/
│       ├── handlers/
│       │   ├── order_created.py       # ← From async interface events
│       │   ├── order_shipped.py
│       │   └── order_cancelled.py
│       └── sqs_consumer.py            # ← From interpretation: async = sqs
│
├── commerce-gateway/                  # Rust (per interpretation override)
│   ├── src/
│   │   └── schema/
│   │       ├── customer.graphql       # ← From interface: expose Customer
│   │       └── order.graphql          # ← From interface: expose Order
│   └── supergraph.yaml
│
├── infrastructure/
│   ├── sqs-queues.tf                  # ← From async interfaces + interpretation
│   └── cockroachdb.tf                 # ← From persistence + interpretation
│
└── documentation/
    └── book/
        ├── src/
        │   ├── architecture.md        # ← Generated from spec
        │   └── flows/
        │       └── checkout.md        # ← Generated from flow definition
        └── book.toml
```

Every file above is derived from the spec + profile. No manual wiring.

## Implementation Strategy

### Phase 1: AML Parser + Model Slice

Build the foundation without changing any archetypes:
- Define the AML YAML schema formally (JSON Schema)
- Implement AML parser in Rust (archetect-core)
- Implement model validation
- Implement model slicing (per-boundary view with expanded entities)
- Implement `archetect.model` Lua module for querying slices
- Test with the commerce example model

Deliverable: `archetect validate architecture.yaml` works. Lua scripts can load and query model slices.

### Phase 2: Archetype Capabilities + Profile Resolution

Build the dispatch mechanism:
- Add `capabilities` section to archetype.yaml schema
- Implement profile format and loader
- Implement convention-based archetype resolution
- Add capabilities to existing p6m archetypes (java-spring-boot-grpc-service, etc.)

Deliverable: Given a model + profile, the system can determine which archetype handles each boundary.

### Phase 3: Orchestrator Engine

Build `archetect generate`:
- DAG resolution and topological sort
- Model slice preparation per boundary
- Sequential archetype dispatch with model context
- Cross-cutting generation (infrastructure, shared libraries)

Deliverable: `archetect generate architecture.yaml --profile p6m-java` generates a full architecture.

### Phase 4: Model-Aware Archetypes

Enhance existing archetypes to consume model slices:
- Start with java-spring-boot-grpc-service (most mature, most used)
- Entity-driven CRUD generation (proto, JPA entities, repositories, service impls)
- Client stub generation from outbound interfaces
- Event publisher/consumer generation from async interfaces
- Extend to other languages/frameworks

Deliverable: Generated services have real entity models, not just id/name defaults.

### Phase 5: Ecosystem

- UI editor for AML (visual architecture design)
- MCP integration (AI-assisted model construction)
- Profile marketplace (community profiles for different stacks)
- Model composition (import shared entity libraries)

## Open Questions

1. **Profile location.** Where do profiles live? Global config? Per-project? Published as git repos?
2. **Model versioning.** How does the model evolve alongside generated code? Do we track what was generated from what model version?
3. **Partial regeneration.** Can you update one boundary without regenerating everything? (Probably Phase 5.)
4. **Custom boundary types.** The assessor type in p6m is domain-specific. How do profiles handle types that aren't in the standard set?
5. **Flow-driven generation.** How much orchestrator boilerplate can we actually generate from flow definitions? What's the right level of abstraction?
6. **Events schema.** Should events have their own schema (like proto messages) or be derived from entity state changes?
