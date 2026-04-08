# ATL Engine Evolution Plan

## Context

The ATL (Archetect Template Language) engine is in good shape — see
`docs/plans/unwrap-hardening-and-atl-audit.md` for the prior audit. This
plan addresses the next round of improvements identified during the
follow-up audit: footgun fixes, filter arguments, additional built-in
primitives (dates/UUIDs/etc.), explicit-only context resolution,
includes, manifest configuration for templating extensions, and template
ergonomics.

The driving principles:

1. **No surprises.** You get what you ask for. No implicit aliasing, no
   silent fallbacks that mask author errors.
2. **Templating ergonomics matter.** Raw Lua is the escape hatch, but
   the common case (interpolate, loop, conditional, filter) should feel
   like a real templating language.
3. **Filters are first-class.** They take arguments, they compose, they
   can be defined by users.
4. **The manifest declares what the engine loads.** Includes for
   templates and library directories for scripts both belong in
   `archetect.yaml`, not implicit conventions.

## Phase Order Rationale

Phases are ordered so that each one is independently shippable, tests
exist before the change is observable, and later phases build on
earlier primitives. The footgun fixes go first because they change
existing behavior — getting them in early means fewer archetypes are
written against the wrong semantics.

Within each phase: **change → tests → docs → ship**. Don't bundle
phases unless they're trivially small.

---

## Phase 1: Footgun Fixes

**Goal:** Eliminate the two behaviors that produce wrong output silently.

### 1.1 Nil renders as empty string, not `"nil"`

**Current behavior** (`compiler.rs:19`):
```lua
local __w = function(s) __out[#__out+1] = tostring(s) end
```
A missing context variable resolves to Lua `nil`, `tostring(nil)` returns
`"nil"`, and the literal string `nil` is written to the output. The test
`test_missing_context_var_renders_empty` documents this.

**New behavior:**
```lua
local __w = function(s)
    if s ~= nil then
        __out[#__out+1] = tostring(s)
    end
end
```

Strict mode (Phase 6) will *also* offer fail-on-undefined as an opt-in.
This change is the *default* — silently dropping a nil is far less
harmful than emitting the literal `"nil"` into a generated source file.

**Test updates:**
- `test_missing_context_var_renders_empty` — assert output is `"Hello !"`
  (not `"Hello nil!"`).
- New test `test_explicit_nil_renders_empty` — `ctx:set("name", nil)` →
  `{{ name }}` produces empty string.

### 1.2 Non-string filter coercion

**Current behavior** (`modules.rs:407-413`):
```rust
let s = match &value {
    Value::String(s) => s.to_string_lossy().to_string(),
    other => format!("{:?}", other),
};
```
`{{ count | upper_case }}` where count is an integer renders something
like `Integer(5)`. The Debug format leaks Rust enum syntax into output.

**Fix:** replace `format!("{:?}", other)` with proper Lua-side coercion.
Use `Value::Integer(i) => i.to_string()`, `Value::Number(n) => …`,
`Value::Boolean(b) => …`, etc. For tables and userdata, error out
explicitly: case filters operate on scalars, not collections.

**Test:**
- `test_filter_coerces_integer_to_string` — `{{ count | upper_case }}`
  with count=5 produces `"5"`.
- `test_filter_rejects_table_input` — `{{ items | upper_case }}` with
  `items` a table produces a clear error, not garbled output.

### 1.3 Remove implicit kebab/snake aliasing

**Current behavior** (`context.rs:83-98`): every `Context::to_lua_table`
call writes snake_case and kebab_case variants of every key into the
template context, in addition to the original key.

**Why it's wrong:** Cases are now an explicit, opt-in concept via the
`cases` opts on prompts (`Cases.programming()`, `Cases.snake()`, etc.).
The implicit aliasing in `to_lua_table` predates that system. It's
invisible — an author seeing `ctx:set("project-name", "foo")` in a
script does not expect `{{ project_name }}` to also work in templates.

**Fix:** delete the alias-writing loop. `to_lua_table` writes only the
keys that exist in the Context. If an author wants `project_name`,
they ask for it via `cases = Cases.programming()` at prompt/set time.

**Migration impact:** any production archetype that *implicitly* relied
on this aliasing will break. We need to:
1. Grep production archetypes for variables that don't exist as
   set/prompted keys but match snake/kebab variants of one that does.
2. Document the change loudly in release notes.
3. Provide a clear error message: when strict mode (Phase 6) lands and
   an undefined variable is accessed, the error should suggest "did
   you mean to declare this case in `cases =` on the prompt?"

**Test:**
- `test_no_implicit_kebab_alias` — `ctx:set("project-name", "foo")` →
  `{{ project_name }}` renders empty (or errors in strict mode), only
  `{{ project-name }}` works.

### 1.4 Lua context table cached on Context

**Current behavior:** every file render in a directory walk re-builds
the Lua table from `Context.data`. For a directory with 200 files,
that's 200 deep copies of the same data.

**Fix:** lazy cache on `Context`. The cache is invalidated whenever
`Context::set` or any `prompt_*` method mutates `data`. Rebuilt on the
next `to_lua_table` call.

This is structurally separate from the surface-area changes above, so
it can ship in 1.4 alongside the rest of Phase 1, or be deferred. It
has no behavioral impact, only perf. **Defer to Phase 8** unless the
audit shows it's a bottleneck on real archetypes.

### Verification

- All existing template_engine tests pass with updated expectations
- New tests above pass
- `cargo test` workspace clean
- Run a representative production archetype end-to-end and visually
  diff output against the v3 baseline

---

## Phase 2: Filter Arguments

**Goal:** Filters take parameters. `{{ name | truncate(40) }}` works.

This is the single biggest expressivity unlock. Without it, custom
filters registered via `template.register_filters` are nearly useless.

### 2.1 Tokenizer/parser changes

The existing `split_filters` in `tokenizer.rs:195` already respects
parens, brackets, and string literals when splitting on `|`. The
remaining work is parsing each filter segment into name + args.

**New `Filter` struct:**
```rust
#[derive(Debug, Clone, PartialEq)]
pub struct Filter {
    pub name: String,
    pub args: Vec<String>,   // raw Lua expressions, passed through verbatim
}
```

**Parsing rules:**
- Bare name: `snake_case` → `Filter { name: "snake_case", args: vec![] }`
- With args: `truncate(40, "...")` → `Filter { name: "truncate", args: vec!["40", "\"...\""] }`
- Args are raw Lua expressions split on top-level commas (respecting
  nested parens, brackets, strings — same logic as `split_filters`).
- Args are NOT pre-evaluated. They're substituted directly into the
  generated Lua, so `{{ x | default(other_var) }}` works — `other_var`
  resolves through `_ENV → __ctx` like any expression.

**New parser helper:** `parse_filter(segment: &str, line: usize) -> Result<Filter, TemplateCompileError>`.

### 2.2 Compiler changes

`apply_filters` in `compiler.rs:178` becomes:
```rust
fn apply_filters(expr: &str, filters: &[Filter]) -> String {
    let mut result = expr.to_string();
    for filter in filters {
        if filter.args.is_empty() {
            result = format!("__filters.{}({})", filter.name, result);
        } else {
            result = format!("__filters.{}({}, {})", filter.name, result, filter.args.join(", "));
        }
    }
    result
}
```

The Lua syntax pre-validator (already in place) catches malformed args
at compile time.

### 2.3 Built-in filter signature changes

Existing inflection filters (`snake_case`, `upper_case`, etc.) take
zero extra args — they keep working without changes.

New filters that *use* arguments are introduced in Phase 3.

### 2.4 Custom filter registration

`template.register_filters` already accepts arbitrary Lua functions.
With filter args, those functions can now take parameters:
```lua
template.register_filters({
    truncate = function(s, n, suffix)
        suffix = suffix or "..."
        if #s <= n then return s end
        return s:sub(1, n) .. suffix
    end
})
```

No code changes needed — the existing registration plumbing already
forwards multi-arg functions.

### 2.5 Tests

- `test_filter_with_single_arg` — `{{ name | truncate(5) }}`
- `test_filter_with_multiple_args` — `{{ name | replace("a", "b") }}`
- `test_filter_arg_resolves_context_var` — `{{ name | default(fallback) }}`
  where `fallback` is set in context.
- `test_filter_arg_with_string_literal` — `{{ items | join(", ") }}`
- `test_filter_arg_with_nested_paren` — `{{ x | f(g(y)) }}`
- `test_filter_chain_with_args` — `{{ name | truncate(10) | upper_case }}`

---

## Phase 3: Additional Built-in Primitives

**Goal:** Templates have access to dates, times, UUIDs, and a few other
common building blocks for code generation. **And** every built-in
behaves consistently in two equivalent forms — bare function call and
filter pipe — so authors are never forced to switch styles.

### 3.0 Filter/function symmetry — the principle

Today, case-conversion functions live in `__filters.snake_case` (used
by `{{ name | snake_case }}`) AND in script-side `Cases.snake()` (used
in Lua scripts), but neither is reachable from inside `{{ }}` *as a
function call*. You cannot write `{{ snake_case(entity.name) }}`. This
is an avoidable inelegance — the same Rust function should be reachable
both ways with one implementation.

**Rule for every built-in introduced in this phase, and retroactively
for the existing case filters:**

> If a function is registered in the filter table, it is also exposed
> as a bare function in the template's `_ENV` under the same name, and
> vice versa. One implementation, two surface forms. Authors pick the
> form that reads better at the call site.

**Why both forms matter:**

```
{{ entity.name | snake_case | upper_case }}     -- pipe reads left-to-right
{{ upper_case(snake_case(entity.name)) }}       -- function reads inside-out
{{ default(maybe_value, "fallback") }}          -- args are easier when nested
{{ items | join(", ") }}                        -- pipe wins when input dominates
```

**Implementation:**

A single registration helper takes a Rust function and a name, and
inserts it into both:
1. The `__filters` Lua table (existing path).
2. The `_ENV` builtins block emitted by `compiler.rs:20-44`.

The compiler's `_ENV` initializer becomes data-driven from a registry
of `(name, lua_function)` pairs rather than the current hard-coded
list of Lua stdlib names. Built-ins are appended to this registry at
Lua VM setup time, before any template is compiled.

**Retroactive fix for existing case filters:**

The existing inflection filters (`snake_case`, `camel_case`, etc.) get
the same treatment in this phase. After Phase 3 ships,
`{{ snake_case(name) }}` works alongside `{{ name | snake_case }}` for
every case style.

**Tests for the principle (apply to every built-in added in 3.1-3.6):**
- For each builtin `foo`, both `{{ foo(x) }}` and `{{ x | foo }}`
  produce the same output (when both forms are sensible).
- For each builtin with args `bar(a, b)`, both `{{ bar(x, a, b) }}`
  and `{{ x | bar(a, b) }}` produce the same output.

### 3.1 Date/time

**Functions** (always callable inside `{{ }}`):
- `now()` — current local datetime as RFC3339 string
- `now_utc()` — current UTC datetime as RFC3339 string
- `today()` — current date as YYYY-MM-DD
- `year()` — current year as integer
- `timestamp()` — current Unix timestamp as integer

**Filters** (transform an existing value):
- `{{ value | date(format) }}` — format a date string with strftime-style
  format. e.g., `{{ today() | date("%Y") }}` → `"2026"`.
- `{{ value | duration_humanize }}` — `90` seconds → `"1m 30s"`
  (skip if not requested by users)

**Implementation:** wrap `chrono` (already a dependency via source.rs).
Module file: `archetect-core/src/script/lua/template_engine/builtins/datetime.rs`.
Registered in both `_ENV` (compiler) and `__filters` (modules.rs).

**Tests:**
- `test_now_returns_rfc3339_string` — output parses as RFC3339
- `test_year_returns_integer` — output is current year
- `test_date_filter_formats_string`

### 3.2 UUIDs

**Functions:**
- `uuid()` — alias for `uuid_v4()`, the default
- `uuid_v4()` — random v4 UUID string
- `uuid_v7()` — time-ordered v7 UUID string (useful for sortable IDs)
- `uuid_nil()` — `00000000-0000-0000-0000-000000000000`

**Implementation:** add `uuid` crate dependency to `archetect-core`
with `v4` and `v7` features. Module file:
`archetect-core/src/script/lua/template_engine/builtins/uuid.rs`.

**Tests:**
- `test_uuid_v4_format` — output matches v4 UUID regex
- `test_uuid_v7_format` — output matches v7 UUID regex
- `test_uuid_default_is_v4` — `uuid()` and `uuid_v4()` produce
  same-format output (different values)

### 3.3 Random

**Functions:**
- `random_int(min, max)` — inclusive integer range
- `random_string(length)` — alphanumeric string
- `random_choice(list)` — pick one element from a list

**Tests:**
- `test_random_int_in_range`
- `test_random_string_length`
- `test_random_choice_from_list`

### 3.4 Environment

These already exist as the `env` module accessible from scripts. Mirror
them into templates so `{{ env.user }}` and `{{ env.os }}` work without
the script having to inject them into the context first.

- `env.user` — current OS username
- `env.os`, `env.arch`, `env.family` — already in scripting engine
- `env.cwd` — process working directory
- `env_var(name, default?)` — read an environment variable with optional
  fallback (NOT a free-for-all `getenv` — explicit name only, gated by
  the security/allow_env policy if we add one)

**Security note:** environment variable access is power. Document the
gate and consider whether unrestricted env access should require an
opt-in like `security.allow_env` similar to existing `allow_exec`.

### 3.5 Path manipulation

For templates that generate config files referencing paths:
- `path_join(a, b, c, ...)` — OS-agnostic join
- `basename(p)`, `dirname(p)`, `extname(p)`
- `path_normalize(p)`

These are filters too: `{{ some_path | basename }}`.

### 3.6 String utilities (filter form)

Now that filters take args, add the workhorse string filters:
- `{{ s | default("fallback") }}` — replace nil/empty with fallback
- `{{ s | truncate(n, suffix?) }}`
- `{{ s | replace(old, new) }}`
- `{{ s | trim }}`, `{{ s | trim_start }}`, `{{ s | trim_end }}`
- `{{ s | indent(n) }}` — prefix every line with n spaces
- `{{ s | repeat(n) }}`
- `{{ s | split(sep) }}` — returns array
- `{{ list | join(sep) }}` — array to string
- `{{ list | length }}`, `{{ list | first }}`, `{{ list | last }}`
- `{{ list | sort }}`, `{{ list | reverse }}`
- `{{ list | unique }}`

**Implementation:** these are mostly thin wrappers over Lua/Rust
stdlib. Group in `builtins/strings.rs`, `builtins/collections.rs`.

### 3.7 Tests for the whole phase

Each builtin gets at least one happy-path test. Edge cases for
arg validation (wrong type, missing required arg) get a test that
asserts a clear error message.

---

## Phase 4: Includes

**Goal:** Templates can compose. `{% include "partials/license.atl" %}`
inlines another template at the same context.

### 4.1 Syntax

```
{% include "path/to/template.atl" %}
```

The path is **relative to the includes directory** declared in the
manifest (Phase 5). NOT relative to the including template — that
would make refactoring brittle.

Whitespace trim markers work: `{%- include "x" -%}`.

### 4.2 Tokenizer changes

The existing tokenizer already produces a `Token::Logic { code }` for
`{% ... %}`. Inside the compiler, before passing logic blocks straight
through to Lua, intercept `include "path"` and handle it specially.

Alternative: introduce a new token type `Token::Include { path }`. This
is cleaner and lets the tokenizer reject malformed include syntax with
a tokenizer error rather than a downstream Lua parse error.

**Decision:** introduce `Token::Include { path: String, line: usize }`.
The tokenizer recognizes `{% include "..." %}` as a special form.

### 4.3 Compiler changes

When the compiler sees `Token::Include { path }`, it must:
1. Resolve the path against the configured includes directory.
2. Read the file (with proper UTF-8 + IO error propagation).
3. Recursively compile the included template's body **inline** —
   it shares the same `__ctx`, `__filters`, `__out`, `__w`.
4. Detect cycles. Maintain a `Vec<Utf8PathBuf>` of currently-being-compiled
   includes; if the new path is already in the stack, return
   `TemplateCompileError::IncludeCycle { path, stack }`.

**Note:** this means the compiler grows a `&IncludeResolver` parameter.
The resolver knows the includes directory and the cycle stack.

### 4.4 New error variants

```rust
TemplateCompileError::IncludeNotFound { path: String, line: usize },
TemplateCompileError::IncludeReadError { path: String, source: io::Error, line: usize },
TemplateCompileError::IncludeCycle { path: String, stack: Vec<String> },
```

### 4.5 Caching

The template cache (`render.rs:17`) is keyed by file path. Includes
should also be cache-keyed — if `partials/license.atl` is included by
50 generated files, it should compile once and inline into each. The
inlining happens at compile time, so each *outer* template ends up with
the include's compiled code embedded — separate cache entries per outer
template, but the include itself is read+tokenized once.

**Actually:** since the include is fully inlined at compile time, the
read-and-tokenize work *does* happen per outer template that uses it.
That's fine for correctness; if it becomes a perf issue we add a
read-cache keyed by include path. **Defer the read-cache.**

### 4.6 Tests

- `test_include_basic` — outer template includes a partial, output is
  the partials's content interpolated against the outer ctx
- `test_include_uses_outer_context` — partial references `{{ name }}`,
  outer sets it
- `test_include_in_loop` — `{% for x in xs %}{% include "row.atl" %}{% end %}`
- `test_include_not_found` — clear error
- `test_include_cycle_detected` — A includes B includes A, error
  identifies the cycle
- `test_nested_include` — A includes B includes C (no cycle), all
  three contribute output

---

## Phase 5: Manifest Configuration Overhaul

**Goal:** `archetect.yaml` declares everything the engine loads:
template includes, scripting library directories, and per-engine
behavior toggles. No more implicit conventions buried in code.

### 5.1 New manifest schema

```yaml
description: "My Archetype"
authors: ["Author Name"]

requires:
  archetect: "3.0.0"

scripting:
  main: "archetype.lua"          # default if file exists
  modules: "modules"             # author's own Lua modules (current behavior)
  libraries:                     # NEW: shared libraries provided by archetypes
    - "lib/utils"                # extra dirs added to Lua's package.path
    - "lib/codegen"

templating:
  content: "."                   # source root for `directory.render`
  templates: "templates"         # source root for `template.render`
  includes: "includes"           # NEW: where {% include %} resolves paths
  undefined: "lenient"           # NEW: lenient | strict (Phase 6)
  trim_blocks: false             # NEW: Jinja-style trim_blocks
  lstrip_blocks: false           # NEW: Jinja-style lstrip_blocks
```

### 5.2 Field-by-field rationale

#### `scripting.libraries`

Today, `scripting.modules` points to one directory of additional Lua
files. Authors who want to share library code across archetypes have
no clean way to do it — they have to bundle the library inside each
archetype's `modules/`.

`scripting.libraries` is a **list of additional directories** appended
to Lua's `package.path` so that `require("utils.casing")` finds them.
Paths are relative to the archetype root.

Future enhancement (out of scope for this plan): `libraries` entries
could be Git URLs or catalog references that pull shared library
archetypes into the cache like normal source resolution. This is the
right composition story long-term but doesn't need to land in Phase 5.

#### `templating.includes`

This is what `{% include %}` resolves against. A single directory by
default; could grow to a list later if a use case appears. Default
value: `"includes"`. If the directory doesn't exist and no template
uses `{% include %}`, no error.

#### `templating.undefined`

Replaces the existing `UndefinedBehavior` enum (currently used by
MiniJinja, soon to be removed). Two values:
- `lenient` (default) — undefined vars render as empty
- `strict` — undefined vars are a render error

(`chainable` was a MiniJinja-specific concept; drop it.)

#### `templating.trim_blocks` / `lstrip_blocks`

Jinja-style whitespace controls. Off by default. When enabled:
- `trim_blocks` strips the first newline after a block tag
- `lstrip_blocks` strips leading whitespace on lines containing only a
  block tag

These reduce the need for explicit `{%-` / `-%}` markers in
heavily-blocked templates.

### 5.3 Migration

The current `TemplatingConfig` has `content`, `templates`,
`undefined_behavior`. New fields are added with sensible defaults so
existing manifests continue to parse. The `UndefinedBehavior::Chainable`
variant gets removed; any manifest using it gets an error message
suggesting `lenient` or `strict`.

### 5.4 Wiring

- `ArchetypeManifest.templating().includes_directory()` →
  `Utf8PathBuf` resolved against archetype root.
- This path is passed into the `IncludeResolver` constructed in
  `register_lua_directory_module` and threaded into the compiler.
- `scripting.libraries` paths are converted to absolute paths and
  prepended to Lua's `package.path` during the Lua VM setup in
  `register_all`.

### 5.5 Tests

- `test_manifest_parses_new_fields`
- `test_manifest_default_includes_directory_is_includes`
- `test_lua_libraries_added_to_package_path`
- `test_include_resolves_against_includes_directory`
- `test_legacy_chainable_undefined_behavior_errors_clearly`

---

## Phase 6: Strict Mode

**Goal:** Authors who want fail-fast on undefined variables can opt in
via `templating.undefined: strict`.

### 6.1 Implementation

The compiler emits a different `_ENV` metatable:
```lua
local _ENV = setmetatable({...builtins...}, {
    __index = function(_, k)
        local v = __ctx[k]
        if v == nil then
            error("undefined template variable: " .. k, 2)
        end
        return v
    end
})
```

Plumbing: `TemplateCompiler::compile(template, name, opts)` accepts an
`opts: CompileOptions` struct that includes `strict: bool`. The
`directory.render` and `template.render` callsites pull this from the
manifest and pass it through.

### 6.2 Error mapping

The Lua error `undefined template variable: foo` is caught in
`render.rs` and mapped to a new variant
`RenderError::UndefinedVariable { name: String, path: Utf8PathBuf }`.

### 6.3 Tests

- `test_strict_mode_errors_on_undefined`
- `test_strict_mode_allows_explicit_nil` — `ctx:set("x", nil)` is
  defined-as-nil and should NOT error in strict mode (nil is a value,
  undefined is the absence of a key)
- `test_strict_mode_default_filter_handles_undefined` — actually no,
  `default` only handles values that exist. Undefined still errors.
  This is the right semantics: `default` is for present-but-empty,
  not for absent.

(That last point may be controversial. The alternative is
`{{ x | default("foo") }}` swallows undefined errors, which is what
Jinja does. Pick one and document it. Recommendation: match Jinja —
`default` handles both nil and undefined — because the alternative
makes `default` much less useful in strict mode.)

---

## Phase 7: Templating Sugar

**Goal:** Common patterns that today require Lua syntax get terse forms
that compile to the same Lua.

### 7.1 `for` sugar

| Sugar                          | Compiles to                          |
|--------------------------------|--------------------------------------|
| `{% for item in items %}`      | `for _, item in ipairs(items) do`    |
| `{% for k, v in items %}`      | `for k, v in pairs(items) do`        |
| `{% for i, item in items %}`   | `for i, item in ipairs(items) do`    |

The compiler/tokenizer detects `for IDENT in EXPR` (single var) and
`for IDENT, IDENT in EXPR` (two vars) at the start of a `Logic` block
and rewrites it. Anything else falls through unchanged — raw Lua still
works.

The two-var form is ambiguous: `for k, v in t` could mean `pairs(t)`
or `ipairs(t)` depending on `t`. **Decision:** `for k, v` defaults to
`pairs` (treats t as a map); `for i, item` (where the first var is `i`,
`idx`, `index`, or `_`) is heuristic-detected as `ipairs`. If the
heuristic is wrong, the author writes raw Lua.

Actually the heuristic is fragile. Cleaner: `for i, item in items`
always means `pairs(items)`, and authors who want ordered iteration
write `for i, item in ipairs(items)` explicitly. That's one keystroke
over today's situation but unambiguous. **Adopt this rule.**

Reconsidered table:

| Sugar                            | Compiles to                          |
|----------------------------------|--------------------------------------|
| `{% for item in items %}`        | `for _, item in ipairs(items) do`    |
| `{% for k, v in items %}`        | `for k, v in pairs(items) do`        |
| `{% for i, item in ipairs items %}` | `for i, item in ipairs(items) do` |

Single-var means "I want each value in order" → ipairs. Two-var means
"I want key-value pairs" → pairs. Explicit `ipairs` for ordered
indexed iteration.

### 7.2 `if` / `elseif` / `else` / `end` (already work, document them)

These work today via raw Lua passthrough. Add explicit examples to docs
showing all three forms.

### 7.3 `set` sugar

```
{% set name = "value" %}
```
compiles to
```lua
local name = "value"
```

This is sugar for declaring a local in the template. `{{ name }}`
afterward resolves to the local first (Lua scoping), then `__ctx`.

### 7.4 Whitespace control via manifest (Phase 5 ties in here)

`trim_blocks` and `lstrip_blocks` from Phase 5 land in the compiler here.

### 7.5 Tests

- `test_for_sugar_single_var_uses_ipairs`
- `test_for_sugar_two_var_uses_pairs`
- `test_for_sugar_explicit_ipairs`
- `test_set_sugar_declares_local`
- `test_set_sugar_visible_to_subsequent_expressions`
- `test_trim_blocks_strips_first_newline`
- `test_lstrip_blocks_strips_block_only_lines`

---

## Phase 8: Integration & Performance Cleanups

**Goal:** Loose ends from the audit that aren't blocking but improve
the architecture.

### 8.1 Cache Lua context table on Context

Deferred from Phase 1. Build the Lua table once per Context, invalidate
on mutation, reuse across all `template.render` and `directory.render`
calls within a single archetype run.

### 8.2 Move template engine out of `script/lua/`

Promote `archetect-core/src/script/lua/template_engine/` →
`archetect-core/src/templating/atl/` (or similar). The engine is
useful independently of the script engine, and the current location
suggests otherwise.

This is a pure refactor — file moves and import updates.

### 8.3 Replace `__atl_filters` global with closure capture

`modules.rs:381-382` uses a Lua global to thread the filter table
through to `template.register_filters`. Replace with `Rc<RefCell<Table>>`
captured in both closures.

### 8.4 Threading `_name` into compile errors

`TemplateCompiler::compile(template, _name)` ignores `_name`. After
Phase 4 (includes) lands, this becomes important — error messages
should say *which* template failed to compile. Wire it through into
all compile error variants and `InvalidLuaSyntax`.

### 8.5 Tests

- `test_context_lua_table_cache_invalidates_on_set`
- `test_template_engine_module_path_under_templating` (the move)

---

## Cross-Cutting: Documentation

Each phase ships with doc updates. The audit revealed that several
elegant behaviors (the `_ENV` trick, kebab-case rewriter, filter
chains) are undocumented in the source. Plan: every public surface
gets a doc comment, and a single `docs/specs/atl-language.md` is
written that defines:

- Syntax (delimiters, expressions, logic, comments, includes, sugar)
- Semantics (variable resolution, undefined behavior, scoping)
- Built-in functions and filters (Phase 3)
- Manifest configuration (Phase 5)
- Custom filter API
- Error catalog

This spec is written incrementally as phases land — don't write it all
upfront, write the section for each phase as that phase ships.

---

## Risks & Open Questions

### Backwards compatibility

The Phase 1 changes are behavior changes, not surface changes.
Production archetypes that relied on:
- Implicit kebab/snake aliasing
- `nil` rendering as the literal `"nil"`

…will produce different output after Phase 1. We need to:
1. Run the Phase 1 changes against the production archetype catalog
   in isolation and audit every diff.
2. Decide whether to gate via a manifest version (`requires.archetect: "3.1.0"`
   gets new behavior, `"3.0.0"` gets legacy) or to do a hard cut and
   migrate the catalog.

Recommendation: hard cut. Backwards-compat shims for footgun fixes are
themselves footguns.

### Filter arg evaluation order

Filter args are inlined as raw Lua expressions and evaluated each time
the filter chain is invoked. For pure expressions (literals, variable
lookups) this is correct. For impure expressions (`uuid()`,
`now()`), it means `{{ x | default(uuid()) }}` generates a fresh UUID
on every evaluation, even if `x` is non-nil and the default is unused.
This matches Jinja behavior. Document it.

### `include` security

If a template can include any path, an archetype could try
`{% include "/etc/passwd" %}`. The includes resolver MUST restrict
paths to the archetype's includes directory and reject `..`, absolute
paths, and symlinks pointing outside. Use `restrict_path` semantics
similar to `modules.rs:711`.

### Library loading security

`scripting.libraries` adds directories to Lua's `package.path`. These
paths are inside the archetype directory, which the user has
implicitly trusted by running the archetype. No additional gating
needed beyond rejecting absolute paths and `..` in the manifest values.

### `default` filter semantics in strict mode

Already discussed in Phase 6. Recommend matching Jinja: `default`
swallows undefined-variable errors. Document loudly.

---

## Sequencing Summary

1. **Phase 1** — Footgun fixes (nil, coercion, no implicit aliasing). Hard cut.
2. **Phase 2** — Filter arguments. Foundation for everything in Phase 3+.
3. **Phase 3** — Built-in primitives (dates, UUIDs, strings, paths).
4. **Phase 4** — Includes.
5. **Phase 5** — Manifest configuration overhaul (includes dir, libraries, undefined, whitespace).
6. **Phase 6** — Strict mode.
7. **Phase 7** — Templating sugar (`for`, `set`, whitespace control wired up).
8. **Phase 8** — Integration & performance cleanups.

Each phase is independently shippable. Phase 1 should land soon to
minimize the number of archetypes written against the wrong semantics.
Phases 2-3 should land together if possible — filter args without
filters that use them is unsatisfying. Phases 4-5 likewise pair up
naturally because includes need a place to live.
