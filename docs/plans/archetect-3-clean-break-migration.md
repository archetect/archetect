# Archetect 3: Clean Break Migration Plan

## Context

Archetect is a Rust code generator used at Ybor (Platform as a Service). The v3 codebase carries v2 legacy: Rhai scripting engine (28 modules, ~4,600 lines), vendored MiniJinja (15,000+ lines), and vendored inquire fork (10,000+ lines). Today we built ATL (Archetect Template Language) — a Lua-native template engine that eliminates MiniJinja for model-driven generation. All 73 production p6m archetypes already have `.archetype3` directories with Lua scripts. The v2 binary at `/Users/jimmie/personal/archetect/archetect` still works for legacy `.rhai` archetypes.

**Goal:** Remove all v2 legacy code, make AML (Architecture Modeling Language) first-class in the Rust object model, add interactive entity/field prompting, and produce clean publishable crates with no vendored forks (except archetect-inflections which has legitimate reasons).

## Target Crate Structure

```
archetect-bin            CLI (clap), `generate` subcommand
archetect-core           Lua engine, ATL rendering, archetype/catalog loading
archetect-aml            NEW: AML parser, model structs, DAG resolver, slicer
archetect-api            IO traits, message types, ContextMap type
archetect-mcp            MCP stdio server
archetect-terminal-io    Terminal IO driver (upstream inquire, not vendored)
archetect-inflections    String transforms (stays vendored — legitimate)
archetect-validations    Validation types
xtask                    Build automation
```

**Removed:** `archetect-templating` (MiniJinja), `archetect-terminal-prompts` (inquire fork)

**Post-migration dependency graph:**
```
archetect-bin
├── archetect-core
│   ├── archetect-api         (ContextMap, IO traits)
│   ├── archetect-aml         (model structs, parser, DAG)
│   ├── archetect-terminal-io (upstream inquire)
│   ├── archetect-inflections (vendored, legitimate)
│   ├── archetect-validations
│   └── mlua                  (Lua 5.4)
├── archetect-mcp
│   └── archetect-core
└── xtask
```

---

## Phase 0: ContextMap — Replace rhai::Map (Foundation)

**Goal:** Introduce a Rust-native `ContextMap` type to replace `rhai::Map`/`rhai::Dynamic` as the universal data interchange format. No functionality changes — just swap the type with bidirectional bridge converters.

**Why first:** `rhai::Map` is the deepest entanglement — it's in `RenderContext`, `Archetype::render()` return type, `Context` (Lua userdata), `RenderArchetypeInfo`, `TestHarnessBuilder`, MCP server, and catalog manifests. Every subsequent phase depends on this being clean.

**The new type** (in `archetect-api`):
```rust
pub type ContextMap = BTreeMap<String, ContextValue>;

pub enum ContextValue {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Array(Vec<ContextValue>),
    Map(ContextMap),
    Nil,
}
```

**Steps:**
1. Define `ContextValue`/`ContextMap` in `archetect-api/src/context_map.rs` with Serialize/Deserialize
2. Add bidirectional `rhai::Map <-> ContextMap` converters in `archetect-core` (temporary bridge)
3. Change `RenderContext::answers` from `rhai::Map` to `ContextMap`
4. Change `Archetype::render()` return type from `rhai::Dynamic` to `ContextMap`
5. Change `Context::ContextValue::Dynamic(rhai::Dynamic)` to `ContextValue::Map(ContextMap)`
6. Add `Context::to_context_map()` alongside existing `to_rhai_map()` / `to_lua_table()`
7. Update MCP server, TestHarnessBuilder, RenderArchetypeInfo
8. Rhai engine continues to work via bridge converters

**Critical files:**
- `archetect-api/src/` — new `context_map.rs`
- `archetect-core/src/archetype/render_context.rs` — answers type change
- `archetect-core/src/script/lua/context.rs` — ContextValue variant change
- `archetect-core/src/archetype/archetype.rs` — render() return type
- `archetect-mcp/src/server.rs` — answers construction

**Verification:** `cargo test` passes (all 130 tests). Both Rhai and Lua archetypes render. MCP server works.

---

## Phase 1: Remove Rhai Engine

**Goal:** Delete the entire Rhai scripting engine. `.rhai` archetypes produce a clear error directing users to v2.

**Steps:**
1. Delete `archetect-core/src/script/rhai/` (28 files, ~4,600 lines)
2. Remove `ScriptEngine::Rhai` (or make it emit error)
3. Remove `render_rhai()` from `archetype.rs`
4. Remove `rhai::Map <-> ContextMap` bridge converters from Phase 0
5. Remove `rhai` dependency from Cargo.toml
6. Move reusable utilities (archive functions) out of Rhai module tree
7. Delete all Rhai test files and test archetypes
8. Clean up imports across codebase (~56 `use rhai::` statements)

**Verification:** `cargo test` passes. Lua archetypes work. `archetect3 render some.rhai-archetype` emits helpful error.

---

## Phase 2: Remove Vendored MiniJinja

**Goal:** Delete `archetect-templating`. All archetypes use ATL.

**Pre-condition:** Verify all 73 production archetypes work with ATL (most use simple `{{ var }}` interpolation which is syntax-compatible).

**Steps:**
1. Change `TemplateEngine` default from `Jinja` to `Lua`
2. Remove `create_environment()` from `script/mod.rs`
3. Remove `Environment<'static>` parameter threading through `execute()`, `register_all()`, etc.
4. Remove MiniJinja rendering path (`render_directory`, `render_contents` in `archetype.rs`)
5. Delete `archetect-templating/` crate (15,000+ lines)

**Verification:** All archetypes render via ATL. Port key MiniJinja test inputs to ATL tests.

---

## Phase 3: Replace Vendored Inquire

**Goal:** Replace vendored inquire 0.6.0 fork with upstream `inquire` crate.

**Steps:**
1. Audit vendored fork customizations (custom List prompt, help enhancements)
2. Add upstream `inquire = "0.7"` to `archetect-terminal-io`
3. Port terminal prompt handlers to upstream API
4. Port catalog's `Select` usage
5. Delete `archetect-terminal-prompts/` crate (10,000+ lines)
6. If customizations can't be replicated: evaluate `dialoguer`, contribute upstream, or build minimal prompt library

**Verification:** Interactive prompts work. MCP unaffected (never touches inquire).

---

## Phase 4: AML First-Class in Object Model

**Goal:** Create `archetect-aml` crate — Rust structs for the AML model, replacing the pure-Lua `aml.lua` library.

**Steps:**
1. Create `archetect-aml/` crate with types: `AmlModel`, `Entity`, `Field`, `FieldType`, `Boundary`, `Interface`, `Flow`, `TypeDef`, `Relation`
2. Implement YAML deserialization with field shorthand normalization
3. Implement model validation (reference integrity, ownership, cycle detection)
4. Implement DAG resolver and topological sort
5. Implement model slicer (per-boundary view)
6. Implement case expansion using `archetect-inflections`
7. Register `archetect.model` as a Rust-backed Lua module
8. Pure-Lua `aml.lua` remains as fallback for existing archetypes

**Verification:** `aml-builder.archetype3` works with Rust-backed module. Commerce model generates correctly.

---

## Phase 5: Interactive Model Prompting

**Goal:** Build AML models interactively through prompts — "Add entity?", "Field name:", "Type:", etc.

**Approach:** Build as a Lua library (`archetect.model.builder`) using existing prompt primitives. No new ScriptMessage variants needed — compose models through `prompt_text`, `prompt_select`, `prompt_confirm` loops.

```lua
-- The archetype decides how to get the model:
local model = archetect.model.load(path)           -- from file
    or archetect.model.from_context(context)        -- from answers/MCP
    or archetect.model.interactive(context)          -- build interactively
```

All three produce the same model object. The archetype doesn't care.

**Verification:** An archetype interactively builds a 2-entity model, then generates from it.

---

## Phase 6: Catalog Redesign

**Goal:** Simplify catalogs for v3. Integrate with AML profiles.

**Changes:**
- Simplify format: entries are `source` + `description` + optional `answers`/`switches`
- Route catalog selection through IO protocol (enables MCP catalog browsing)
- Add AML profile integration (select profile → select archetype pattern)
- Update all catalogs to `requires: archetect: "3.0.0"`

---

## Phase 7: TUI Model Designer (Stretch)

**Goal:** Ratatui-based visual model constructor.

- Entity boxes with field lists
- Drag entities into service boundaries
- Draw relationship arrows
- Export to AML YAML or feed directly into generation

Independent of other phases after Phase 4. Can develop in parallel.

---

## Phase Dependencies

```
Phase 0 (ContextMap) ─── MUST BE FIRST
   │
   ├── Phase 1 (Remove Rhai)
   │
   ├── Phase 2 (Remove MiniJinja) ── can parallel with Phase 1
   │
   └── Phase 3 (Replace inquire) ── independent, can parallel
          │
Phase 4 (AML crate) ── after Phase 1 ideally
   │
   ├── Phase 5 (Interactive prompting)
   ├── Phase 6 (Catalog redesign) ── also needs Phase 3
   └── Phase 7 (TUI designer) ── stretch goal
```

Phases 1, 2, 3 can run in parallel after Phase 0. Phase 4 starts during/after Phase 1.

---

## Migration Verification Strategy

**Side-by-side:** v2 binary at `/Users/jimmie/personal/archetect/archetect` handles `.rhai` archetypes. v3 binary handles `.archetype3` Lua archetypes. Both are available throughout migration.

**Test corpus:** The 73 production `.archetype3` directories at `/Users/jimmie/work/p6m-archetypes/`. Representative set for smoke testing:
- `org-prompts.archetype3` (leaf, shared by 25+ archetypes)
- `java-spring-boot-grpc-service.archetype3` (builder, 4+ components)
- `transactional-architecture-builder.archetype3` (largest builder, 9 components)
- `rust-grpc-service.archetype3` from archetect-aml (ATL + entity iteration)

**After each phase:** `cargo test` (all workspace tests), render representative archetypes in headless mode, MCP smoke test.

---

## Risk Assessment

| Risk | Level | Mitigation |
|------|-------|------------|
| Phase 0 ContextMap touches everything | **High** | Small commits, keep rhai bridge alive, test after each file |
| ATL may not handle all MiniJinja patterns | **Medium** | Port MiniJinja test suite before Phase 2 |
| Inquire fork has unlisted customizations | **Medium** | Audit diff against upstream before Phase 3 |
| Catalog YAML serde compatibility | **Medium** | Test with real catalog files after ContextMap change |
| Rhai engine removal is mechanical | **Low** | Phase 0 isolates it behind bridge |

---

## What This Unlocks

After all phases:
- **Clean publishable crates** — no vendored forks cluttering the dependency tree
- **Lua all the way down** — one language for orchestration and templates
- **AML as a first-class citizen** — Rust-typed models, not loosely-typed Lua tables
- **Interactive model building** — design architectures through prompts or TUI
- **MCP-native catalog browsing** — AI agents can discover and select archetypes
- **~30,000 lines of dead code removed** (Rhai + MiniJinja + inquire fork)
