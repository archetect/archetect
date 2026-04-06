# Archetect 3: Lua as Primary Scripting Engine

## Motivation

Archetect v2 uses Rhai as its embedded scripting language. While Rhai works, it has significant limitations for archetype authors:

- **No real LSP/IDE support** — Rhai's LSP is experimental. No autocomplete, no error detection, no hover docs.
- **Niche language** — Almost no one knows Rhai outside of Rust embedding. Every archetype author must learn it from scratch.
- **Silent failures** — Property access on maps doesn't error on typos (`context.pnpm_intall` silently returns `()`).
- **Tiny ecosystem** — No external libraries available. Self-contained by design.
- **Single maintainer risk** — Rhai's long-term maintenance is uncertain compared to Lua's decades of stability.
- **Limited async story** — Relevant for the planned CodegenExtension/server architecture.

Lua addresses all of these while being simple enough for the same use case.

## Current Rhai Usage Analysis

### What archetype scripts actually do

Analysis of ~80 production archetypes in p6m-archetypes shows scripts are **simple configuration + orchestration**, not general-purpose programming:

| Feature | Usage | Notes |
|---------|-------|-------|
| Maps `#{}` | 100% | Core data structure |
| `prompt()` with casing | ~95% | Main user-facing API |
| Case conversions | ~85% | `PROGRAMMING_CASES` pattern pervasive |
| `if`/`for` control flow | ~80% | Basic conditionals |
| Archetype composition | ~70% | `Archetype("name")` |
| Git operations | ~50% | Publishing pattern |
| Shell execution | ~30% | Advanced scripts only |
| Module imports | ~1% | Almost never used |
| Closures/FP | 0% | Not observed |

Scripts peak at ~170 lines. No recursion, no closures, no higher-order functions.

### Observed pain points in production scripts

1. **YAML embedded as backtick strings** — fragile, error-prone structured data building
2. **Typos silently pass** — `context.pnpm_intall` doesn't error; logic silently fails
3. **No IDE support** — authors write scripts blind, debug with `display(as_yaml(context))`
4. **Verbose case configuration** — `CasedIdentityCasedValue(PROGRAMMING_CASES)` repeated everywhere
5. **Inconsistent key naming** — kebab-case vs snake_case in same scripts, no convention enforcement
6. **No error handling** — scripts assume everything succeeds; no try/catch used anywhere

### Rhai integration surface (~4,160 lines)

17 registered modules in `archetect-core/src/script/rhai/modules/`:

| Module | Lines | Complexity | Lua Equivalent Effort |
|--------|-------|------------|----------------------|
| prompt_module (+ 5 submodules) | ~1,390 | High — casing system, 7 prompt types, 4 overloads | Medium — single function with table arg |
| cases_module | ~480 | Medium — 14 case transforms + strategies | Easy — same functions, different registration |
| git_module | ~850 | Medium — 10+ operations, multiple overloads | Easy — table args replace overloads |
| archive_module | ~275 | Low | Easy |
| github_module | ~267 | Low — async wrapped in blocking | Easy — mlua has native async |
| directory_module | ~213 | Medium — state threading | Medium |
| archetype_module | ~208 | Medium — nested rendering | Medium |
| exec_module | ~197 | Low | Easy |
| set_module | ~118 | Medium — casing integration | Medium |
| path_module | ~110 | Low | Easy |
| formats_module | ~55 | Low | Easy |
| pair_module | ~46 | Low | Easy |
| utils_module | ~42 | Low | Easy |
| log_module | ~41 | Low | Easy |
| archetect_module | ~145 | Low — static info | Easy |
| render_module | ~30 | Low | Easy |
| rand | ~100 | Low | Easy |

**Key Rhai-specific features used:**
- Function overloading (prompt has 4 signatures) — Lua uses single function with table args instead
- Custom types (Path, Pair, Directory, ArchetypeFacade) — Lua uses UserData trait via mlua
- Map `+=` merge operator — Lua would use a `merge()` helper or table unpacking
- `Dynamic` type system — Lua's dynamic typing is native
- Module resolver (FileModuleResolver + StaticModuleResolver) — Lua's `require()` system is more mature
- `NativeCallContext` for error position tracking — mlua provides similar via error metadata

## Rhai vs Lua Comparison (for Archetect's use case)

| Dimension | Rhai | Lua (mlua) | Winner |
|-----------|------|-----------|--------|
| User familiarity | Niche | Widely known | **Lua** |
| LSP/IDE support | Experimental | lua-language-server (1M+ installs) | **Lua** |
| String interpolation | Native `${...}` | `string.format()` or `..` | **Rhai** |
| Function overloading | Native | Table args (more idiomatic) | Tie |
| Ecosystem/libraries | Self-contained, tiny | Massive (LuaRocks) | **Lua** |
| Performance | AST walker (slowest) | Bytecode + optional LuaJIT | **Lua** |
| Sandboxing | Built-in, configurable | Manual, requires setup | **Rhai** |
| Rust integration | Simple register_fn | More boilerplate (create_function) | **Rhai** (slightly) |
| Async support | Not built-in | First-class via mlua async feature | **Lua** |
| Long-term stability | Single maintainer | Decades of proven stability | **Lua** |
| Contributor pool | Tiny | Large | **Lua** |
| Coroutines | Not available | Native — ideal for prompt/response | **Lua** |
| Type annotations | None | LuaLS annotation files | **Lua** |

### What Lua gains that isn't obvious

**Coroutines** — Lua coroutines could simplify the IO protocol significantly. Instead of the channel-pair threading model, the script `coroutine.yield()`s when it needs a prompt response, and the host `coroutine.resume()`s it with the answer. This is a natural fit for Archetect's sequential prompt→response flow and much simpler than the current `request()`/`response()` channel pair.

**LuaLS annotations** — You can ship `.lua` annotation files that give archetype authors full autocomplete for the Archetect API (prompt, render, set, git_init, etc.) without any special IDE plugin. Just `---@param message string` style annotations.

**Luau option** — Meta's Luau variant (supported by mlua) adds gradual typing. Could optionally offer typed archetypes for teams that want more safety.

### What Lua loses (and mitigations)

**String interpolation** — Rhai's backtick `${...}` is nicer than Lua's `string.format()`. Mitigation: `render("{{ var }}", context)` already exists and is the primary template mechanism. Could also provide a small `f()` helper: `f("Hello {name}", context)`.

**Function overloading** — Rhai's `prompt()` with 4 signatures is elegant. In Lua, use a single function with optional table argument:

```lua
-- Rhai v2 style (4 overloads):
context += prompt("Label:")
context += prompt("Label:", "key")
context += prompt("Label:", #{ type: Select([...]) })
context += prompt("Label:", "key", #{ type: Select([...]) })

-- Lua v3 style (single function, table arg):
context = merge(context, prompt("Label:"))
context = merge(context, prompt("Label:", { key = "name" }))
context = merge(context, prompt("Label:", { key = "name", type = Select({...}) }))
```

This is actually more Lua-idiomatic and arguably cleaner — the settings table replaces positional args.

**Sandboxing** — Requires manual environment restriction. Mitigation: standard Lua pattern — create a restricted environment at engine creation that only exposes Archetect's API functions. Well-documented, widely practiced.

**Map merge operator** — Rhai's `context += prompt(...)` is concise. Lua equivalent: `merge(context, prompt(...))` or `context = prompt("Label:", { into = context })`. Could also provide a method-style API: `context:prompt("Label:", {...})`.

## Proposed Design for Archetect 3

### Dual-engine strategy: two distinct scripting APIs

The Rhai and Lua engines are **not** ports of each other. They are two different scripting APIs:

- **Rhai engine** — Frozen v2 compatibility layer. Runs existing archetypes exactly as-is. No API changes. Maintenance-only.
- **Lua engine** — Clean-slate v3 API. Redesigned from scratch, informed by v2 pain points. The recommended path for all new archetypes.

`archetype.yaml` gains: `scripting.engine: lua` (default) or `scripting.engine: rhai`

### Script engine abstraction

```
archetect-core/src/script/
├── mod.rs              # ScriptEngine trait
├── lua/                # Lua engine (primary, new v3 API)
│   ├── mod.rs          # mlua engine creation
│   └── modules/        # Redesigned API modules
└── rhai/               # Rhai engine (frozen v2 compat)
    ├── mod.rs
    └── modules/        # Existing v2 modules, unchanged
```

```rust
trait ScriptEngine {
    fn execute(
        &self,
        archetype: &Archetype,
        archetect: &Archetect,
        render_context: &RenderContext,
    ) -> Result<(), ArchetypeError>;
}
```

### Lua v3 API Design: What Changes From v2

This is not a transliteration of Rhai into Lua syntax. It's a redesign that fixes v2's pain points.

#### 1. Context object replaces loose maps + merge

**v2 Rhai (loose maps, manual merge):**
```rhai
let context = #{};
context += prompt("Project Name:", "project-name", #{
    cases: [CasedIdentityCasedValue(PROGRAMMING_CASES)],
});
context += set("description", "A service", #{
    cases: [FixedKeyCasedValue("Description", TitleCase)],
});
```

**v3 Lua (Context object with methods):**
```lua
local ctx = Context.new()

ctx:prompt("Project Name:", {
    key = "project-name",
    cases = Cases.programming(),  -- replaces verbose CasedIdentityCasedValue(PROGRAMMING_CASES)
})

ctx:set("description", "A service", {
    cases = { Cases.fixed("Description", "title") },
})
```

Why: The `Context` object owns the data and the operations. No more `merge()` / `+=` boilerplate. Methods return `self` for chaining if desired. The context is a first-class concept, not an anonymous map.

#### 2. Simplified case system

**v2 Rhai (verbose, confusing names):**
```rhai
cases: [
    CasedIdentityCasedValue(PROGRAMMING_CASES),
    FixedKeyCasedValue("organization-title", TitleCase),
    FixedKeyCasedValue("OrganizationTitle", TitleCase),
]
```

**v3 Lua (readable, intention-clear):**
```lua
cases = Cases.programming()                          -- standard set: pascal, camel, snake, kebab, train, constant
cases = Cases.all()                                   -- every case variant
cases = { Cases.fixed("organization-title", "title") } -- explicit fixed-key variant
cases = Cases.set("pascal", "camel", "snake")         -- custom set
```

Why: `CasedIdentityCasedValue` is unreadable. The four strategy types (`CasedIdentityCasedValue`, `CasedKeyCasedValue`, `FixedIdentityCasedValue`, `FixedKeyCasedValue`) collapse into a simple API where the common case (`Cases.programming()`) is one call, and custom cases are explicit.

#### 3. Prompt types as clean constructors

**v2 Rhai (type field in settings map):**
```rhai
context += prompt("Persistence:", "persistence", #{
    type: Select(["None", "JPA", "JDBC"]),
    defaults_with: "JPA",
});
```

**v3 Lua (dedicated prompt methods):**
```lua
ctx:select("Persistence:", "persistence", {"None", "JPA", "JDBC"}, {
    default = "JPA",
})

ctx:text("Description:", "description", { placeholder = "Enter a description" })
ctx:confirm("Enable metrics?", "metrics", { default = true })
ctx:list("Tags:", "tags", { min = 1, max = 5 })
ctx:multi_select("Features:", "features", {"Auth", "Metrics", "Tracing"})
ctx:int("Port:", "service-port", { default = 8080, min = 1024, max = 65535 })
ctx:editor("Notes:", "notes")
```

Why: Dedicated methods are discoverable via LSP autocomplete. The `type: Select(...)` pattern requires knowing the magic string/constructor. Dedicated methods have typed parameters and clear documentation. The key is always the second arg (no more positional ambiguity from v2's 4 overloads).

#### 4. Rendering API

**v2 Rhai (overloaded render function):**
```rhai
render(Directory("base"), context);
render(Directory("persistence"), "subdir", context);
render(Archetype("child"), context);
render("{{ name | snake_case }}", context);  // inline template
```

**v3 Lua (explicit, namespaced):**
```lua
-- Directory rendering
archetype.render_directory("base", ctx)
archetype.render_directory("persistence", ctx, { destination = "subdir", if_exists = "preserve" })

-- Archetype composition
archetype.render_component("child", ctx)
archetype.render_component("child", ctx, {
    switches = {"feature-a"},
    use_defaults = {"port"},
})

-- Inline template rendering
local result = template.render("{{ name | snake_case }}", ctx)
```

Why: `render()` doing four different things based on the first argument type is confusing and undiscoverable. Separate functions make the intent clear, are individually documentable, and have distinct LSP signatures.

#### 5. Git/GitHub as a module, not globals

**v2 Rhai (global functions, path as first arg):**
```rhai
git_init(context["project-name"]);
git_add_all(context["project-name"]);
git_commit(context["project-name"], "Initial commit");
git_remote_add(context["project-name"], "origin", url);
git_push(context["project-name"]);
gh_repo_create(org .. "/" .. name, Private);
```

**v3 Lua (Git repo object):**
```lua
local repo = git.init(ctx:get("project-name"), { branch = "main" })
repo:add_all()
repo:commit("Initial commit")
repo:remote_add("origin", url)

if github.repo_exists(org .. "/" .. name) == false then
    github.create_repo(org .. "/" .. name, { visibility = "private" })
end

repo:push("origin", "main")
```

Why: The v2 pattern repeats the project path in every call. A repo object captures it once. Method calls are chainable and discoverable. GitHub operations are namespaced separately from Git.

#### 6. Structured logging and output

**v2 Rhai (inconsistent globals):**
```rhai
display("message");    // what's the difference?
print("message");      // ...from this?
log(Info, "message");  // and this?
debug(context);        // debugging aid
```

**v3 Lua (clear separation):**
```lua
log.info("Processing service: %s", name)    -- structured logging with format strings
log.debug("Context: %s", inspect(ctx))      -- debug-level
log.warn("Deprecated feature used")
log.error("Failed to create directory")

output.print("Generated project: " .. name)  -- user-facing output (always shown)
output.banner("Setting up workspace")         -- prominent display
```

Why: v2 conflates logging (for debugging) with output (for the user). Clear namespacing makes intent obvious.

#### 7. Execution and shell commands

**v2 Rhai:**
```rhai
execute("npm", #{ args: ["install"], directory: project_dir });
let output = capture("node", #{ args: ["--version"] });
```

**v3 Lua:**
```lua
shell.run("npm", {"install"}, { cwd = project_dir })
local version = shell.capture("node", {"--version"})
shell.run("chmod", {"+x", "gradlew"}, { cwd = project_dir })
```

Why: `execute` vs `capture` is unclear. `shell.run` and `shell.capture` are self-documenting. Args as a direct table parameter instead of inside a settings map.

#### 8. Archives

**v2 Rhai:**
```rhai
zip(source, destination);
tar_gz(source, destination);
```

**v3 Lua (essentially the same, namespaced):**
```lua
archive.zip(source, destination)
archive.tar_gz(source, destination)
archive.tar(source, destination)
```

#### 9. Switches and answers

**v2 Rhai (globals):**
```rhai
if switch_enabled("debug-answers") {
    display(as_yaml(ANSWERS));
}
let val = ANSWERS["some-key"];
```

**v3 Lua (Context methods):**
```lua
if archetype.switch("debug-answers") then
    log.debug("Answers: %s", inspect(ctx))
end

-- Answers are pre-loaded into context, accessible directly
local val = ctx:get("some-key")

-- Or check if an answer was provided
if ctx:has("some-key") then ... end
```

#### 10. Error handling (new in v3)

**v2 Rhai:** No error handling. Scripts assume everything succeeds.

**v3 Lua:**
```lua
-- Protected calls for operations that might fail
local ok, err = pcall(function()
    shell.run("npm", {"install"}, { cwd = project_dir })
end)
if not ok then
    log.warn("npm install failed: %s", err)
    output.print("Run 'npm install' manually after generation")
end

-- Or use the built-in error function to abort with a message
if not ctx:has("required-field") then
    error("required-field must be provided")
end
```

### Complete v3 Script Example

```lua
-- archetype.lua (v3 Lua API)
local ctx = Context.new()

-- Project identity
ctx:text("Organization Prefix:", "org-prefix", {
    cases = Cases.programming(),
    placeholder = "acme",
})

ctx:text("Project Name:", "project-name", {
    cases = Cases.programming(),
    placeholder = "my-service",
})

ctx:set("project-title",
    template.render("{{ org_prefix }}-{{ project_name }}", ctx),
    { cases = Cases.programming() }
)

-- Technology choices
ctx:select("Persistence:", "persistence", {"None", "JPA", "JDBC"}, {
    default = "JPA",
})

ctx:select("API Framework:", "api-framework", {"REST", "gRPC", "GraphQL"}, {
    default = "REST",
})

ctx:multi_select("Features:", "features", {
    "Authentication", "Metrics", "Tracing", "Health Checks",
}, { default = {"Metrics", "Health Checks"} })

-- Conditional port configuration
ctx:int("Service Port:", "service-port", { default = 8080, min = 1024, max = 65535 })
ctx:set("management-port", ctx:get("service-port") + 1)

-- Render base project
archetype.render_directory("base", ctx)

-- Conditional rendering based on choices
if ctx:get("persistence") ~= "None" then
    archetype.render_directory("persistence-common", ctx)
    archetype.render_directory("persistence-" .. ctx:get("persistence"):lower(), ctx)
end

archetype.render_directory("api-" .. ctx:get("api-framework"):lower(), ctx)

-- Render selected features
for _, feature in ipairs(ctx:get("features")) do
    archetype.render_component("feature-" .. feature:lower():gsub(" ", "-"), ctx)
end

-- Git publishing
if archetype.switch("git-publish") then
    local repo = git.init(ctx:get("project-title"), { branch = "main" })
    repo:add_all()
    repo:commit("Initial commit from archetype")

    local repo_path = ctx:get("org-prefix") .. "/" .. ctx:get("project-title")
    if not github.repo_exists(repo_path) then
        github.create_repo(repo_path, { visibility = "private" })
    end
    repo:remote_add("origin", "git@github.com:" .. repo_path .. ".git")
    repo:push("origin", "main")

    output.print("Published to: https://github.com/" .. repo_path)
else
    output.print("To publish, re-run with --switch git-publish")
end
```

### LuaLS Annotation Files

Shipped with Archetect, giving archetype authors full IDE support:

```lua
---@meta archetect

---@class Context
---@field new fun(): Context
local Context = {}

---Prompt for text input and store in context
---@param message string Prompt message displayed to the user
---@param key string Key to store the result under
---@param opts? {placeholder?: string, default?: string, help?: string, min?: integer, max?: integer, optional?: boolean, cases?: CaseSpec[]}
function Context:text(message, key, opts) end

---Prompt for selection from a list
---@param message string Prompt message
---@param key string Key to store the result under
---@param options string[] Available options
---@param opts? {default?: string, help?: string, cases?: CaseSpec[]}
function Context:select(message, key, options, opts) end

---Prompt for boolean confirmation
---@param message string Prompt message
---@param key string Key to store the result under
---@param opts? {default?: boolean, help?: string}
function Context:confirm(message, key, opts) end

---Prompt for integer input
---@param message string Prompt message
---@param key string Key to store the result under
---@param opts? {default?: integer, min?: integer, max?: integer, help?: string}
function Context:int(message, key, opts) end

---Prompt for a list of strings
---@param message string Prompt message
---@param key string Key to store the result under
---@param opts? {min?: integer, max?: integer, help?: string, cases?: CaseSpec[]}
function Context:list(message, key, opts) end

---Prompt for multiple selections
---@param message string Prompt message
---@param key string Key to store the result under
---@param options string[] Available options
---@param opts? {default?: string[], min?: integer, max?: integer, help?: string}
function Context:multi_select(message, key, options, opts) end

---Prompt for text via editor
---@param message string Prompt message
---@param key string Key to store the result under
---@param opts? {default?: string, help?: string}
function Context:editor(message, key, opts) end

---Set a value directly in the context
---@param key string Key to store under
---@param value any Value to store
---@param opts? {cases?: CaseSpec[]}
function Context:set(key, value, opts) end

---Get a value from the context
---@param key string Key to retrieve
---@return any
function Context:get(key) end

---Check if a key exists in the context
---@param key string
---@return boolean
function Context:has(key) end

---@class Cases
Cases = {}

---Standard programming cases: pascal, camel, snake, kebab, train, constant
---@return CaseSpec[]
function Cases.programming() end

---All available case variants
---@return CaseSpec[]
function Cases.all() end

---Custom set of case styles
---@param ... string Case style names
---@return CaseSpec[]
function Cases.set(...) end

---Fixed key with specific case
---@param key string The fixed key name
---@param style string The case style to apply
---@return CaseSpec
function Cases.fixed(key, style) end

---@class archetype
archetype = {}

---Render a directory template to the destination
---@param dir string Directory name within the archetype
---@param ctx Context Template context
---@param opts? {destination?: string, if_exists?: "overwrite"|"preserve"|"prompt"}
function archetype.render_directory(dir, ctx, opts) end

---Render a child archetype component
---@param name string Component name (from archetype.yaml components)
---@param ctx Context Template context
---@param opts? {switches?: string[], use_defaults?: string[], use_defaults_all?: boolean}
function archetype.render_component(name, ctx, opts) end

---Check if a switch is enabled
---@param name string Switch name
---@return boolean
function archetype.switch(name) end

---@class template
template = {}

---Render an inline Jinja2 template string
---@param tmpl string Template string
---@param ctx Context|table Template context
---@return string
function template.render(tmpl, ctx) end

---@class git
git = {}

---Initialize a git repository
---@param path? string Path (relative to destination). Defaults to destination root.
---@param opts? {branch?: string}
---@return GitRepo
function git.init(path, opts) end

---@class GitRepo
local GitRepo = {}
function GitRepo:add(pattern) end
function GitRepo:add_all() end
function GitRepo:commit(message) end
function GitRepo:branch(name) end
function GitRepo:checkout(name) end
function GitRepo:remote_add(name, url) end
function GitRepo:push(remote, branch) end

---@class github
github = {}

---Check if a repository exists
---@param repo string Repository in "owner/name" format
---@return boolean
function github.repo_exists(repo) end

---Create a new repository
---@param repo string Repository in "owner/name" format
---@param opts? {visibility?: "public"|"private"|"internal"}
function github.create_repo(repo, opts) end

---@class shell
shell = {}

---Run a command
---@param program string
---@param args? string[]
---@param opts? {cwd?: string, env?: table<string,string>}
function shell.run(program, args, opts) end

---Run a command and capture stdout
---@param program string
---@param args? string[]
---@param opts? {cwd?: string, env?: table<string,string>}
---@return string
function shell.capture(program, args, opts) end

---@class archive
archive = {}

---@param source string Source path
---@param destination string Output archive path
function archive.zip(source, destination) end
function archive.tar(source, destination) end
function archive.tar_gz(source, destination) end

---@class log
log = {}
function log.info(fmt, ...) end
function log.debug(fmt, ...) end
function log.warn(fmt, ...) end
function log.error(fmt, ...) end
function log.trace(fmt, ...) end

---@class output
output = {}

---Print a message to the user (always visible)
function output.print(message) end

---Display a prominent banner message
function output.banner(message) end
```

### Coroutine-based IO

Instead of the current channel-pair model where `archetect.request()` sends a message and `archetect.response()` blocks on a channel, Lua coroutines offer a simpler model:

```rust
// Host side (simplified)
loop {
    match lua_coroutine.resume(last_response) {
        Ok(CoroutineStatus::Yielded(script_message)) => {
            // Handle prompt, file write, log, etc.
            last_response = handle_message(script_message);
        }
        Ok(CoroutineStatus::Complete) => break,
        Err(e) => return Err(e),
    }
}
```

The script simply calls `ctx:text(...)` which internally yields a prompt message. No threading, no channels, no Arc/Mutex. This naturally supports both CLI (handle inline) and server (serialize and send over network) modes.

This pairs with the IO protocol overhaul: the coroutine yield/resume model is the Lua engine's native way of implementing the ScriptIoHandle/ClientIoHandle protocol. The Rhai engine continues using channels.

## Implementation Considerations

### mlua crate selection

- **Crate**: `mlua` (v0.11+)
- **Lua version**: Lua 5.4 (stable, modern) or Luau (gradual typing)
- **Features**: `vendored` (compile Lua from source, no system dependency), `serialize` (serde integration), `async` (for server mode)

### Migration path for archetype authors

Since the v3 Lua API is intentionally different from v2 Rhai, migration is a rewrite — but the scripts are simple enough (~50-170 lines) that this is straightforward:

1. Ship a `archetect migrate` command that does a best-effort Rhai → Lua translation (handles the mechanical parts: map syntax, control flow, function names)
2. Document the v3 API with full examples and a migration guide showing v2 → v3 patterns side by side
3. Both engines work indefinitely — no forced migration. v2 Rhai archetypes run as-is.
4. New archetypes default to Lua. `archetype.yaml` with no `scripting.engine` field defaults to `lua` in v3.

### Testing strategy

The two engines have different APIs, so they do NOT share the same test suite:

- **Rhai engine**: Existing v2 integration tests run unchanged. Regression-only.
- **Lua engine**: New test suite covering the v3 API. New test archetypes written in Lua.
- **End-to-end validation**: Both engines should be able to generate the same output from equivalent archetypes. A validation suite renders key archetypes through both engines and diffs the output.

## Relationship to Other v3 Plans

This plan pairs with [archetect-3-io-overhaul.md](archetect-3-io-overhaul.md):

- The IO protocol overhaul (ScriptIoHandle/ClientIoHandle, WriteFile, etc.) is engine-agnostic — both Rhai and Lua engines implement the same IO protocol
- Lua coroutines provide the Lua engine's natural implementation of the IO protocol (yield/resume instead of channel pairs)
- The Rhai engine continues using the channel-pair model
- Both plans benefit from a fresh v3 repo where breaking changes are expected

## Open Questions

1. **Lua 5.4 vs Luau?** — Luau adds gradual typing but is less standard. Could support both via mlua feature flags. Luau typing would enhance the LuaLS experience further.
2. **Context as object vs module?** — `ctx:prompt(...)` (method on context) vs `prompt(ctx, ...)` (global function taking context). Object style is cleaner but means every function needs the context.
3. **Coroutine IO model vs channel model?** — Coroutines are simpler but change the execution model. Worth prototyping early.
4. **LuaLS annotations packaging** — Ship with Archetect CLI? Separate `archetect-lua-types` repo? Auto-generated into archetype directories?
5. **Should `archetype.lua` replace `archetype.rhai` as the default script filename?** — Yes, for v3 archetypes. The manifest's `scripting.main` field can override.
