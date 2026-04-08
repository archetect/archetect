# Lua-Native Template Engine

## The Problem

Archetect's model-driven code generation requires iterating structured data in templates: entities with fields, fields with types and constraints, relations with cardinalities and cross-service references. MiniJinja cannot do this because the data conversion boundary between Lua and MiniJinja is lossy:

```
Lua Table → rhai::Dynamic → serde → MiniJinja Value
```

Every hop loses fidelity. The result: archetype scripts resort to building strings in Lua via `table.concat` and passing pre-rendered text to templates. Entity templates have `// TODO: Add fields from domain model` stubs because field-level data simply can't cross the bridge.

This isn't a bug to fix — it's an architectural mismatch. The orchestration language (Lua) and the template language (Jinja) don't share a data model. Workarounds are possible but degrade the authoring experience to the point where templates become dumb string slots.

## The Solution: Lua All the Way Down

Replace MiniJinja with a template engine where:

1. **Templates compile to Lua functions** — The parser (written in Rust) translates template syntax into Lua source code
2. **Templates execute in the same mlua VM** — Zero data conversion. Lua tables from the archetype script flow directly into template logic
3. **The syntax stays familiar** — `{{ expr }}` for interpolation, `{% lua_code %}` for logic blocks. Anyone who's used Jinja/Django/Twig recognizes the shape
4. **The logic IS Lua** — Not a pseudo-language. Real `for`/`end`, real `if`/`then`/`end`, real `ipairs()`, real function calls

## Template Syntax

### Expressions: `{{ }}`

Interpolate a value into the output. Supports dotted access and filter chains.

```
{{ entity.name.pascal }}
{{ field.name | snake_case }}
{{ field | rust_type }}
{{ org_name }}.{{ solution_name }}.{{ project_name }}
```

Filters use the `|` pipe operator. Filters are Lua functions — they receive the left-hand value and return a string.

Filter chaining: `{{ name | snake_case | upper }}` compiles to `filters.upper(filters.snake_case(name))`.

### Logic Blocks: `{% %}`

Raw Lua code embedded in the template. The delimiters are stripped; the content is Lua.

```
{% for _, field in ipairs(entity.local_fields) do %}
    pub {{ field.name.snake }}: {{ field | rust_type }},
{% end %}
```

```
{% if field.required then %}
    @NotNull
{% end %}
```

```
{% if entity.relations and #entity.relations > 0 then %}
// Relations
{% for _, rel in ipairs(entity.relations) do %}
    {{ rel | jpa_annotation }}
    private {{ rel | java_type }} {{ rel.name.camel }};
{% end %}
{% end %}
```

### Comments: `{# #}`

Template comments that don't appear in output.

```
{# This generates the entity struct from the domain model #}
```

### Whitespace Control

Whitespace-trimming markers: `{{- expr -}}`, `{%- code -%}`. A `-` adjacent to the delimiter trims whitespace on that side (same semantics as Jinja2).

```
{% for i, field in ipairs(entity.local_fields) do -%}
    {{ field | proto_type }} {{ field.name.snake }} = {{ i }};
{%- end %}
```

### Raw Blocks

For template content that should not be parsed (e.g., GitHub Actions `${{ }}` expressions):

```
{% raw %}
    steps:
      - uses: actions/checkout@v3
      - run: echo "${{ github.ref }}"
{% endraw %}
```

### Template Blocks/Partials

Templates can call other templates as functions. This replaces Jinja's `{% include %}` and `{% macro %}` with something more natural:

```
{# In the main template #}
{% for _, field in ipairs(entity.local_fields) do %}
{{ partial("field_declaration", { field = field, indent = "    " }) }}
{% end %}
```

The `partial()` function loads and renders another template file with a given context table. Since it's just a Lua function call, archetype authors can also define inline helpers:

```
{% function proto_field(field, index) %}
    {{ field | proto_type }} {{ field.name.snake }} = {{ index }};
{% end %}

{% for i, field in ipairs(entity.local_fields) do %}
{{ proto_field(field, i) }}
{% end %}
```

## Compilation Model

The Rust-side parser tokenizes the template and emits a Lua function. This happens once at load time, not on every render.

### Input

```
syntax = "proto3";

package {{ org_name }}.{{ solution_name }}.{{ project_name }};

message {{ entity.name.pascal }} {
{% for i, field in ipairs(entity.local_fields) do %}
    {{ field | proto_type }} {{ field.name.snake }} = {{ i }};
{% end %}
}
```

### Compiled Lua Output

```lua
return function(__ctx, __filters)
    local __out = {}
    local __w = function(s) __out[#__out+1] = tostring(s) end
    setfenv and setfenv(1, setmetatable(__ctx, {__index = _G}))

    __w("syntax = \"proto3\";\n\npackage ")
    __w(__ctx.org_name)
    __w(".")
    __w(__ctx.solution_name)
    __w(".")
    __w(__ctx.project_name)
    __w(";\n\nmessage ")
    __w(__ctx.entity.name.pascal)
    __w(" {\n")
    for i, field in ipairs(__ctx.entity.local_fields) do
        __w("    ")
        __w(__filters.proto_type(field))
        __w(" ")
        __w(field.name.snake)
        __w(" = ")
        __w(i)
        __w(";\n")
    end
    __w("}\n")

    return table.concat(__out)
end
```

Key points:
- The function receives `__ctx` (the context table) and `__filters` (registered filter functions)
- All `{{ expr }}` become `__w(expr)` calls
- All `{% code %}` becomes inline Lua
- Variable references in `{{ }}` are resolved against `__ctx`
- The compiled function is cached and reused across renders

### Scoping: Template Variables vs Context

Inside `{{ }}` expressions, bare names resolve against the context table. Inside `{% %}` blocks, it's raw Lua — you have the full language, and you access context explicitly via `__ctx` or through the environment setup.

To keep templates clean, the engine sets up the environment so that bare names in `{{ }}` resolve context keys:

```
{{ project_name }}     →  __w(__ctx.project_name)
{{ entity.name.pascal }} →  __w(__ctx.entity.name.pascal)
```

But in logic blocks, full Lua is available:

```
{% local types = require("type_maps") %}
{% local rust_type = types.rust[field.type] or field.type %}
```

## Filter System

### Built-in Filters

All existing MiniJinja inflection filters are ported as Lua functions:

| Filter | Example | Result |
|--------|---------|--------|
| `snake_case` | `{{ "OrderItem" \| snake_case }}` | `order_item` |
| `pascal_case` | `{{ "order-item" \| pascal_case }}` | `OrderItem` |
| `camel_case` | `{{ "order-item" \| camel_case }}` | `orderItem` |
| `kebab_case` | `{{ "OrderItem" \| kebab_case }}` | `order-item` |
| `constant_case` | `{{ "OrderItem" \| constant_case }}` | `ORDER_ITEM` |
| `train_case` | `{{ "order_item" \| train_case }}` | `Order-Item` |
| `title_case` | `{{ "order_item" \| title_case }}` | `Order Item` |
| `upper` | `{{ name \| upper }}` | `ORDER` |
| `lower` | `{{ name \| lower }}` | `order` |
| `pluralize` | `{{ "entity" \| pluralize }}` | `entities` |
| `singularize` | `{{ "entities" \| singularize }}` | `entity` |

These are backed by the existing `archetect-inflections` crate, exposed to Lua.

### Custom Filters

Archetype authors define filters as Lua functions. This is where the power is — filters for model-driven generation are domain-specific:

```lua
-- lib/filters/rust.lua
local M = {}

local type_map = {
    String    = "String",
    Integer   = "i64",
    Long      = "i64",
    Decimal   = "rust_decimal::Decimal",
    UUID      = "Uuid",
    Boolean   = "bool",
    Timestamp = "DateTime<Utc>",
    Date      = "NaiveDate",
    Bytes     = "Vec<u8>",
}

function M.rust_type(field)
    if field.is_relation then
        return "Uuid"  -- FK reference
    end
    return type_map[field.type] or field.type
end

function M.proto_type(field)
    local proto_map = {
        String = "string", Integer = "int64", Long = "int64",
        UUID = "string", Boolean = "bool", Decimal = "string",
        Timestamp = "google.protobuf.Timestamp",
    }
    return proto_map[field.type] or "string"
end

return M
```

Filters are registered per-archetype in the script:

```lua
local rust_filters = require("filters.rust")
template.register_filters(rust_filters)
```

Or globally in the archetype manifest:

```yaml
templating:
  engine: lua
  filters:
    - lib/filters/rust.lua
```

### Filters vs Methods

For complex objects like entities/fields, filters and dotted access complement each other:

```
{{ field.name.snake }}          -- Direct property access (case-expanded name)
{{ field | rust_type }}          -- Filter: takes the whole field object, returns a string
{{ field.type | lower }}         -- Filter on a string property
```

Filters operate on the value to their left. They're ideal for transformations that need the full object context (like type mapping, which depends on `field.type`, `field.is_relation`, etc.).

## What Templates Look Like

### Before (MiniJinja — current)

**Proto template** — can't iterate fields:
```proto
message {{ EntityName }}Response {
    string id = 1;
    // TODO: Add fields from domain model
}
```

**Archetype script** — builds strings in Lua:
```lua
local entity_mods = {}
for _, entity in ipairs(entities) do
    table.insert(entity_mods, "pub mod " .. entity.name.snake .. ";")
end
context:set("entity_mod_declarations", table.concat(entity_mods, "\n"))
```

### After (Lua-native)

**Proto template** — iterates fields directly:
```proto
message {{ entity.name.pascal }}Response {
{% for i, field in ipairs(entity.local_fields) do %}
    {{ field | proto_type }} {{ field.name.snake }} = {{ i }};
{% end %}
{% for _, rel in ipairs(entity.relations) do %}
    string {{ rel.name.snake }}_id = {{ #entity.local_fields + _ }};
{% end %}
}

message Create{{ entity.name.pascal }}Request {
{% for i, field in ipairs(entity.local_fields) do %}
{% if not field.key then %}
    {{ field | proto_type }} {{ field.name.snake }} = {{ i }};
{% end %}
{% end %}
}
```

**Entity struct** — full field generation:
```rust
use serde::{Deserialize, Serialize};
{% local imports = require("rust_imports") %}
{{ imports.for_entity(entity) }}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct {{ entity.name.pascal }} {
{% for _, field in ipairs(entity.local_fields) do %}
    pub {{ field.name.snake }}: {{ field | rust_type }},
{% end %}
{% for _, rel in ipairs(entity.relations) do %}
    pub {{ rel.name.snake }}_id: Uuid,
{% end %}
}

impl {{ entity.name.pascal }} {
    pub fn new(
{% for _, field in ipairs(entity.local_fields) do %}
{% if not field.key and field.required then %}
        {{ field.name.snake }}: {{ field | rust_type }},
{% end %}
{% end %}
    ) -> Self {
        Self {
{% for _, field in ipairs(entity.local_fields) do %}
{% if field.key then %}
            {{ field.name.snake }}: Uuid::new_v4(),
{% elseif field.required then %}
            {{ field.name.snake }},
{% elseif field.default then %}
            {{ field.name.snake }}: {{ field.default }},
{% else %}
            {{ field.name.snake }}: Default::default(),
{% end %}
{% end %}
{% for _, rel in ipairs(entity.relations) do %}
            {{ rel.name.snake }}_id: Uuid::nil(),
{% end %}
        }
    }
}
```

**Wiring files** — no more string building in Lua:
```rust
{# entities/mod.rs #}
{% for _, entity in ipairs(entities) do %}
pub mod {{ entity.name.snake }};
{% end %}
```

```rust
{# grpc/mod.rs #}
{% for _, entity in ipairs(entities) do %}
mod {{ entity.name.snake }}_service;
{% end %}

pub fn register_services(builder: Server) -> Router {
    builder
{% for _, entity in ipairs(entities) do %}
        .add_service(proto::{{ entity.name.snake }}_service_server::{{ entity.name.pascal }}ServiceServer::new(
            {{ entity.name.snake }}_service::{{ entity.name.pascal }}ServiceImpl,
        ))
{% end %}
}
```

**Archetype script** — dramatically simplified:
```lua
local aml = require("aml")

local context = Context.new()
-- ... identity prompts ...

-- Set the full entity list directly into context
-- No string building. No table.concat. Just set the data.
context:set("entities", entities)

-- Render per-entity templates
for _, entity in ipairs(entities) do
    context:set("entity", entity)
    directory.render("contents/entity", context)
end

-- Wiring files — the templates do the iteration now
directory.render("contents/wiring", context)
```

The archetype script goes from 127 lines to ~40. The templates go from dumb string slots to expressive code generators.

### JPA Entity (Java)

```java
package {{ org_name }}.{{ solution_name }}.{{ entity.name.snake }};

import javax.persistence.*;
{% if entity.relations and #entity.relations > 0 then %}
import javax.validation.constraints.*;
{% end %}

@Entity
@Table(name = "{{ entity.name.snake | pluralize }}")
public class {{ entity.name.pascal }} {

{% for _, field in ipairs(entity.local_fields) do %}
{% if field.key then %}
    @Id
    @GeneratedValue(strategy = GenerationType.AUTO)
{% end %}
{% if field.required and not field.key then %}
    @NotNull
{% end %}
{% if field.unique then %}
    @Column(unique = true)
{% end %}
    private {{ field | java_type }} {{ field.name.camel }};

{% end %}
{% for _, rel in ipairs(entity.relations) do %}
{% if rel.cross_service then %}
    // Resolved via {{ rel.target_boundary }}-client
    @Column(name = "{{ rel.name.snake }}_id")
    private UUID {{ rel.name.camel }}Id;
{% else %}
    @{{ rel | jpa_annotation }}
    private {{ rel.target.pascal }} {{ rel.name.camel }};
{% end %}

{% end %}
}
```

## Integration with Archetect

### Engine Selection

The `archetype.yaml` manifest declares which template engine to use:

```yaml
templating:
  engine: lua           # New: Lua-native template engine
  # engine: jinja       # Default: MiniJinja (v2 compat)
  filters:
    - lib/filters/rust.lua
```

When `engine: lua` is specified:
- Template files are parsed and compiled to Lua functions by the Rust parser
- Template functions execute in the same mlua VM as the archetype script
- Context is a Lua table, not a rhai::Map
- Filters are Lua functions, not MiniJinja registrations

When `engine: jinja` (or omitted):
- Existing MiniJinja behavior, unchanged
- Rhai archetypes always use this engine
- Full backwards compatibility

### Rendering API

The Lua rendering API stays the same — `directory.render()`, `template.render()`. The engine selection is transparent to the script:

```lua
-- These work identically regardless of engine
directory.render("contents/entity", context)
template.render("templates/readme.md", context)
```

The difference is what's available inside the templates.

### Context Passing

With the Lua engine, the context passed to templates IS the Lua table. No conversion:

```lua
-- The archetype script sets structured data
context:set("entity", {
    name = { pascal = "Order", snake = "order", camel = "order" },
    local_fields = {
        { name = { snake = "id" }, type = "UUID", key = true },
        { name = { snake = "total" }, type = "Money", required = true },
    },
    relations = {
        { name = { snake = "customer" }, target_entity = "Customer", cross_service = true },
    },
})

-- The template accesses it directly
-- {{ entity.name.pascal }}              → "Order"
-- {% for _, f in ipairs(entity.local_fields) do %} → iterates the actual table
```

### File and Directory Name Rendering

File and directory names containing `{{ }}` expressions are rendered using the same Lua engine:

```
contents/{{ entity.name.snake }}/proto/{{ entity.name.snake }}.proto
```

This means file names can use the same filters and expressions as file contents.

## Implementation Plan

### Phase 1: Template Parser (Rust)

Build the parser that compiles template syntax to Lua source code.

**Input:** Template string with `{{ }}`, `{% %}`, `{# #}` blocks
**Output:** Lua source code (a function definition)

The parser handles:
- Expression tokenization and filter chain compilation
- Logic block passthrough (Lua code is emitted as-is)
- Comment stripping
- Whitespace control (`-` trimming)
- Raw block passthrough
- Source map tracking (template line → Lua line) for error reporting

This is a straightforward recursive descent parser. No ambiguity — the delimiters are unambiguous.

**Deliverable:** `TemplateCompiler::compile(template: &str) -> Result<String>` that produces valid Lua source.

### Phase 2: Runtime Integration

Wire the compiled templates into the mlua VM.

- Load compiled Lua template functions into the VM
- Register built-in filters (inflection functions from `archetect-inflections`)
- Implement `directory.render()` and `template.render()` for the Lua engine path
- Implement `partial()` for template composition
- Implement file/directory name rendering through the Lua engine
- Template caching: compile once, execute many times

**Deliverable:** `directory.render("contents/entity", context)` works with Lua templates.

### Phase 3: Filter Library

Port built-in filters and create the custom filter registration system.

- Expose `archetect-inflections` functions to Lua (snake_case, pascal_case, etc.)
- Implement `template.register_filters(table)` for archetype-defined filters
- Implement manifest-based filter loading (`templating.filters` in archetype.yaml)

**Deliverable:** Both built-in and custom filters work in templates.

### Phase 4: Error Reporting

Map Lua runtime errors back to template source locations.

- The parser tracks a source map: compiled Lua line → template file + line
- When a template function errors, the error message shows the template location, not the compiled Lua
- Include the template line content in error messages

**Deliverable:** `Error in contents/entity/{{ entity-name }}.proto line 7: attempt to index a nil value (field 'local_fields')` — actionable, not cryptic.

### Phase 5: MiniJinja Deprecation Path

- MiniJinja remains the default for `engine: jinja` and all Rhai archetypes
- New Lua archetypes default to `engine: lua`
- No forced migration — both engines coexist indefinitely
- The vendored MiniJinja crate stays but receives no new features

## Design Decisions

### Why not use an existing Lua template library?

- **Lupa, lustache, etlua** — Pure Lua implementations. We can't use them directly because our Lua runs inside mlua (Rust). We'd need to bundle their source and load it into our VM, which is possible but adds a dependency on someone else's parser for a core capability.
- **etlua's `<% %>`** — Angle brackets inside Java/XML/Proto templates would be a readability nightmare.
- **lustache** — Mustache is intentionally logic-less. Model-driven generation needs logic.

The right approach: write the parser in Rust (fast, excellent error reporting, full control), compile to Lua code, execute in our VM. We get Rust's parsing quality with Lua's zero-boundary data access.

### Why compile to Lua instead of interpreting?

1. **Performance** — Compiled Lua functions are JIT-friendly. No interpretation overhead.
2. **Simplicity** — The compiled function is plain Lua. Debug it by printing it.
3. **Full Lua semantics** — No need to implement a Lua interpreter. `{% %}` blocks are real Lua.
4. **Caching** — Compile once, call the function on every render with different context tables.

### Why keep `{{ }}` syntax instead of `<% %>`?

1. **Familiarity** — Jinja/Django/Twig/Mustache/Handlebars all use `{{ }}`. Developers know it.
2. **Visual distinction** — `{{ value }}` for output vs `{% code %}` for logic is clearer than `<%= value %>` vs `<% code %>`
3. **Code template readability** — In a `.java` or `.proto` file, `{{ }}` doesn't conflict with language syntax (unlike `<% %>` in XML/HTML)
4. **Migration** — Existing MiniJinja templates use `{{ }}`. Migration to Lua templates is syntactically minimal.

### Why not just fix the MiniJinja data bridge?

We could fix the `Lua Table → rhai::Dynamic → serde → MiniJinja Value` pipeline. But even if perfectly lossless, we'd still have:
- Two languages to learn (Lua + Jinja)
- Two filter systems to maintain (Lua functions + MiniJinja registrations)
- Two scoping models to reason about
- No ability to call Lua functions from templates
- No ability to `require()` shared libraries in templates

"Lua all the way down" isn't just about fixing the data bridge — it's about eliminating an entire category of complexity.

## Risks and Mitigations

### Risk: Template authoring complexity

Lua syntax in templates is more verbose than Jinja (`for _, x in ipairs(t) do` vs `for x in t`).

**Mitigation:** The verbosity is modest and the payoff is enormous — full Lua semantics, direct data access, custom functions. Archetype authors already write Lua scripts; using Lua in templates is natural, not a new language to learn.

### Risk: Security

Templates can execute arbitrary Lua code, including `require()`, `io.open()`, etc.

**Mitigation:** Archetect already trusts archetype scripts with full Lua execution. Templates run in the same sandbox. This is not a new attack surface — it's the same trust model.

### Risk: Error quality

Template errors show up as Lua runtime errors with compiled-code line numbers.

**Mitigation:** Source maps (Phase 4) translate compiled Lua positions back to template file + line. This is a solved problem — every compile-to-X language does it.

### Risk: Migration effort

Existing v3 Lua archetypes use MiniJinja templates.

**Mitigation:** The set of v3 Lua archetypes is small (this is a new initiative). Migration is syntactically minimal — most `{{ var }}` expressions are identical. Only `{% %}` blocks change from Jinja syntax to Lua syntax. Both engines coexist, so migration is gradual and optional.

## Appendix: Grammar

```
template     = (text | expression | logic | comment | raw)*
text         = <any text not starting with {{ {% {# or {% raw %}>
expression   = '{{' '-'? expr filter* '-'? '}}'
logic        = '{%' '-'? lua_code '-'? '%}'
comment      = '{#' <any text> '#}'
raw          = '{%' 'raw' '%}' <any text> '{%' 'endraw' '%}'
expr         = lua_expression
filter       = '|' identifier ( '(' args ')' )?
identifier   = [a-zA-Z_][a-zA-Z0-9_]*
```

The parser is a simple state machine over the input:
1. Scan for `{{`, `{%`, `{#`, or `{% raw %}`
2. Everything before a delimiter is literal text → emit as string write
3. `{{ expr }}` → parse expression, apply filters, emit as `__w(result)`
4. `{% code %}` → emit code verbatim
5. `{# comment #}` → skip
6. `{% raw %}...{% endraw %}` → emit enclosed text as literal string write
