# CLAUDE.md

## Project Overview

**This is the Archetect 3 repository** — a major version evolution of Archetect, the code-centric, language-agnostic code generator written in Rust.

- **v3 repo**: https://github.com/archetect/archetect-3
- **v2 repo** (stable, production): https://github.com/archetect/archetect
- **Docs** (v2): https://archetect.github.io
- **GitHub Org**: https://github.com/archetect
- **License**: MIT

The codebase currently reflects v2.1.0. v3 work builds on top of it.

## v3 Initiatives

Three planned initiatives, documented in `docs/plans/`:

1. **[IO Protocol Overhaul](docs/plans/archetect-3-io-overhaul.md)** — Route file writes through the IO channel (not direct `std::fs`). Introduce `ScriptIoHandle`/`ClientIoHandle` traits, fallible error handling, WriteFile/WriteDirectory/Complete/Ack/Initialize messages. Enables the closed-source CodegenExtension for COS.

2. **[Lua Scripting Engine](docs/plans/archetect-3-lua-scripting-engine.md)** — Add Lua (via mlua) as the primary scripting engine with a redesigned v3 API. Rhai retained as a frozen compatibility layer for v2 archetypes. The Lua API is not a port of Rhai — it's a clean-slate redesign: `Context` object, dedicated prompt methods (`ctx:text()`, `ctx:select()`), simplified case system (`Cases.programming()`), namespaced modules (`git.init()`, `shell.run()`), LuaLS annotations for full IDE support.

3. **[Warts and Improvements](docs/plans/archetect-3-warts-and-improvements.md)** — Comprehensive catalog of v2 issues: 25+ panic paths via `.unwrap()`, lost script error context, missing manifest validation, no dry-run, no test framework, no component versioning, documentation gaps. Prioritized fix list for v3.

## Related Projects

- **v2 codebase**: `/Users/jimmie/personal/archetect/archetect` — stable production CLI, do not mix with v3 work
- **Production archetypes**: `/Users/jimmie/work/p6m-archetypes` — ~80 Rhai archetypes, the backwards-compat benchmark
- **COS / Onyx**: `/Users/jimmie/personal/jimmiebfulton/onyx` — the CodegenExtension target platform
- **feature/client-server** (v2 branch): Has working gRPC proof-of-concept with the richer IO protocol. Reference implementation for the IO overhaul — don't port verbatim, but use as a guide

## Workspace Structure

Cargo workspace with 9 crates. Dependency graph:

```
archetect-bin (CLI entry point)
├── archetect-core (business logic, scripting, rendering)
│   ├── archetect-api (IoDriver trait, command types)
│   ├── archetect-templating (vendored MiniJinja 0.30.6)
│   ├── archetect-terminal-io (terminal IoDriver impl)
│   │   └── archetect-terminal-prompts (vendored inquire fork)
│   ├── archetect-inflections (case conversions, pluralization)
│   └── archetect-validations (validation error types)
└── xtask (build/install automation)
```

### Crate Purposes

| Crate | Purpose |
|-------|---------|
| `archetect-bin` | CLI (clap), configuration loading (figment), subcommand dispatch |
| `archetect-core` | Archetype/catalog loading, Rhai engine, template rendering, source/cache management |
| `archetect-api` | `ScriptIoHandle`/`ClientIoHandle` traits, prompt/write command structs |
| `archetect-templating` | Vendored MiniJinja — do not update from upstream without care |
| `archetect-terminal-io` | `TerminalIoDriver` bridging prompts to `ScriptIoHandle` |
| `archetect-terminal-prompts` | Vendored inquire fork — interactive terminal prompts |
| `archetect-inflections` | String transforms: camelCase, snake_case, plural/singular, etc. |
| `archetect-validations` | Validation rules and error types |
| `xtask` | `cargo xtask install` with optional `--static-openssl` |

## Build & Development Commands

```bash
# Build
cargo build

# Run tests (entire workspace)
cargo test

# Test specific crate
cargo test -p archetect-core
cargo test -p archetect-templating

# Install CLI locally for manual testing
cargo xtask install

# Run CLI without installing
cargo run -p archetect-bin -- <args>

# Lint and format
cargo clippy --all-targets --all-features
cargo fmt

# Build requires protoc (for gRPC proto compilation in archetect-core/build.rs)
# macOS: brew install protobuf
# Ubuntu: sudo apt install protobuf-compiler
```

## Architecture Deep Dive

### Core Flow: Rendering an Archetype

1. CLI parses args → loads Configuration (merging YAML config + CLI overrides via figment)
2. `Source` resolves the archetype origin (local path or Git URL with caching)
3. `Archetype` loads `archetype.yaml` manifest and validates requirements
4. Rhai engine executes `main.rhai` (or manifest-specified script)
5. Script calls `prompt()` → messages sent via `ScriptIoHandle` → terminal responds
6. Script calls `render_directory()` or `render()` → MiniJinja processes templates
7. Output files written to destination directory

### Key Source Files

| File | What it does |
|------|-------------|
| `archetect-bin/src/main.rs` | CLI entry point, arg parsing, subcommand dispatch |
| `archetect-core/src/archetect/archetect.rs` | `Archetect` struct — main orchestrator, builder pattern |
| `archetect-core/src/archetype/archetype.rs` | `Archetype` — loading, rendering, script execution |
| `archetect-core/src/source.rs` | `Source` — Git/local resolution, caching |
| `archetect-core/src/script/rhai/` | Rhai engine setup and all custom modules |
| `archetect-core/src/catalog/` | Catalog loading and action dispatching |
| `archetect-core/src/configuration/` | Configuration struct, YAML loading, merging |
| `archetect-core/src/system.rs` | `SystemLayout` trait — filesystem layout abstraction |
| `archetect-api/src/io_driver.rs` | `ScriptIoHandle` / `ClientIoHandle` traits |

### Rhai Scripting Modules

All in `archetect-core/src/script/rhai/modules/`:

| Module | Functions |
|--------|-----------|
| `prompt_module` | `prompt()` — text, int, bool, list, select, multiselect, editor |
| `render_module` | `render()` — render single template string |
| `directory_module` | `render_directory()` — render template directory to destination |
| `cases_module` | Case conversions: `CamelCase()`, `snake_case()`, `kebab-case()`, etc. |
| `exec_module` | `exec()` — run shell commands |
| `log_module` | `trace()`, `debug()`, `info()`, `warn()`, `error()`, `display()`, `print()` |
| `path_module` | Path manipulation utilities |
| `git_module` | `git_init()`, `git_add()`, `git_commit()`, `git_branch()`, `git_push()`, etc. |
| `github_module` | `gh_repo_exists()`, `gh_repo_create()` (requires `GITHUB_TOKEN`) |
| `archive_module` | `zip()`, `tar()`, `tar_gz()` — create archives |
| `archetect_module` | `archetect::version()`, `archetect::env::*`, runtime info |
| `archetype_module` | `archetect::archetype::description()`, `::directory()`, etc. |
| `set_module` | Set data structure operations |
| `pair_module` | Pair operations |
| `utils_module` | Miscellaneous utilities |
| `rand` | Random number generation |
| `formats_module` | Format transformations |

### Template System

- Jinja2-compatible syntax: `{{ variable }}`, `{% if %}`, `{% for %}`
- Custom filters from inflections: `{{ name | snake_case }}`, `{{ name | pluralize }}`
- Template files live in archetype's `contents/` or `templates/` directory
- Directory and file names can be parameterized: `{{ artifact_id }}/src/main.rs`

### Configuration

Loaded from (in merge order):
1. Defaults (built-in)
2. `~/.archetect/archetect.yaml` (user config)
3. `~/.archetect/etc.d/*.yaml` (drop-in configs)
4. `.archetect.yaml` (project-local)
5. `--config-file` CLI option
6. CLI flags (`--offline`, `--headless`, answers, switches, defaults)

Key config sections: `actions`, `offline`, `headless`, `answers`, `switches`, `security`, `locals`, `updates`.

### IO Driver Architecture

All script↔user communication flows through `ScriptIoHandle` (trait in `archetect-api`):
- Script sends `ScriptMessage` (prompts, file writes, logs)
- Client responds with `ClientMessage` (string, int, bool, array, ack, abort)
- `TerminalIoDriver` implements this for CLI interaction
- This abstraction allows alternative frontends (tests use `SyncIoDriver`)

### Source & Caching

- Git sources cached in `~/.archetect/cache/`
- Pull timestamps tracked via git notes (`archetect.pulled`)
- Supports branch/tag refs
- Local overrides via `locals` config section
- Cache commands: `archetect cache clear|pull|manage`

## Testing

### Running Tests

```bash
cargo test                          # All workspace tests
cargo test -p archetect-core        # Core crate only
cargo test -p archetect-templating  # Template engine only
cargo test -p archetect-inflections # String inflection tests
```

### Test Structure

- **`archetect-core/tests/`** — Integration tests using `TestHarness` (in `test_utils.rs`)
  - `prompts/` — Test archetypes for each prompt type (text, int, bool, list, multiselect)
  - `utils/` — Utility and switch tests
  - `git/` — Git module integration tests
  - `github/` — GitHub module tests
- **`archetect-templating/tests/`** — 11 test files with `insta` snapshot testing (114+ template inputs)
- **`archetect-inflections/tests/`** — 60+ case conversion test cases
- **Inline `#[cfg(test)]`** modules throughout source files

### TestHarness Pattern

Integration tests use `TestHarness` which:
1. Spawns archetype rendering in a separate thread
2. Uses `SyncIoDriver` for programmatic prompt/response
3. Sends `ClientMessage` responses to prompts
4. Validates `ScriptMessage` outputs
5. Checks `render_succeeded()` status

### Testing with Real Archetypes

Generate projects from published archetypes to verify end-to-end behavior:

```bash
# Simple archetype render (interactive prompts)
cargo run -p archetect-bin -- render https://github.com/archetect/archetype-rust-cli.git /tmp/test-output

# Non-interactive with answers
cargo run -p archetect-bin -- render https://github.com/archetect/archetype-rust-cli.git /tmp/test-output \
  -a project-name=my-project -a description="Test project" -D

# Browse a catalog interactively
cargo run -p archetect-bin -- render https://github.com/archetect/catalog-rust.git /tmp/test-output

# Render from local archetype (e.g., test fixtures)
cargo run -p archetect-bin -- render archetect-core/tests/prompts/text_prompt_scalar_tests /tmp/test-output

# Use --use-defaults-all (-D) for non-interactive CI testing
cargo run -p archetect-bin -- render <source> /tmp/test-output -D
```

**Available test archetypes from GitHub org:**

| Archetype | Description |
|-----------|-------------|
| `archetype-rust-cli` | Basic Rust CLI with clap |
| `archetype-rust-service-tonic-workspace` | gRPC microservice |
| `archetype-rust-service-actix-diesel-workspace` | Actix + Diesel web service |
| `archetect-initializer` | Archetect config initializer |
| `dot-gitignore.archetype` | .gitignore generator |

**Available catalogs:**

| Catalog | Description |
|---------|-------------|
| `catalog-rust` | Rust archetype catalog |
| `catalog-java` | Java archetype catalog |
| `catalog-go` | Go archetype catalog |
| `archetect.catalog` | Master catalog aggregating all |

### Local Test Archetype Structure

Each test archetype in `archetect-core/tests/` follows:
```
test_name/
├── archetype.yaml    # Manifest (description, requires)
├── archetype.rhai    # Script exercising features
└── contents/         # Template files (if any)
```

## Archetype Anatomy (for reference when modifying rendering)

A typical archetype:
```
my-archetype/
├── archetype.yaml          # Manifest: description, authors, requires, scripting, templating config
├── archetype.rhai          # Main Rhai script (or path specified in manifest)
├── rhai/                   # Additional Rhai modules
├── contents/               # Template directory (rendered via render_directory())
│   └── {{ project_name }}/
│       └── src/
│           └── main.rs
└── templates/              # Individual templates (rendered via render())
```

`archetype.yaml` manifest fields:
```yaml
description: "My Archetype"
authors: ["Author"]
languages: ["Rust"]
frameworks: ["Actix"]
tags: ["web", "api"]
requires:
  archetect: "2.0.0"
scripting:
  main: "archetype.rhai"
  modules: "rhai"
templating:
  content_directory: "contents"
  templates_directory: "templates"
  undefined_behavior: "strict"   # strict | lenient | chainable
components:
  child-name:
    source: "https://github.com/org/child-archetype.git"
```

## CI/CD

GitHub Actions workflows in `.github/workflows/`:
- **`build.yml`** — Runs on all branch pushes: `cargo build` + `cargo test` on Ubuntu 24.04 (requires `protobuf-compiler`)
- **`release.yml`** — Runs on tag pushes: cross-platform builds (Linux x64, macOS aarch64, Windows x64), creates GitHub release with archives and checksums

## Version Control

This repository uses **Jujutsu (jj)** for version control. Use `jj` commands instead of `git`:

```bash
jj status          # instead of git status
jj log             # instead of git log
jj diff            # instead of git diff
jj new             # create new change
jj describe -m ""  # set change description
jj bookmark set    # instead of git branch
```

## Project Documentation

- **Specs** go in `docs/specs/` — technical specifications for features and systems
- **Plans** go in `docs/plans/` — implementation plans and design documents

## Dogfooding: creating new archetypes, catalogs, libraries, components

**Always scaffold new Archetect artifacts from our own starters** — do not
hand-write from scratch. If the starter is missing something you need,
**fix the starter first**, then re-scaffold. This keeps the starters
battle-tested and the ecosystem consistent.

| Creating a new... | Use this starter |
|---|---|
| Archetype | `archetect-rust/archetype-starter-archetype` (or `archetect-common/archetype-starter-archetype` — master catalog aliases it under `common/starters/archetype-starter`) |
| Component | `common/starters/component-starter` |
| Catalog | `common/starters/catalog-starter` |
| Library | `common/starters/library-starter` |

### Use the archetect MCP from agent sessions

When working in a Claude Code (or other MCP-capable agent) session,
prefer the `archetect` MCP server over shelling out to `archetect`
directly. It exposes catalog discovery and render as structured tool
calls — better than parsing terminal output.

One-time session registration (via mcp-loader or your MCP manager):

```
transport: stdio
command: archetect
args: ["mcp"]
```

Tools provided:
- `catalog_search { query }` — discover archetypes by keyword (AND terms)
- `catalog_browse { path? }` — walk the catalog tree
- `catalog_render { path, destination, answers?, switches?, use_defaults_all? }` — render by catalog path
- `render { source, destination, ... }` — render from a URL or local path
- `respond { value }` — answer an interactive prompt in an active session
- `cancel` — abort the current render

Users working interactively at the CLI still use `archetect render` directly.

### Typical agent flow

```
catalog_search { query: "archetype starter" }
→ discover archetect/common/starters/archetype-starter
catalog_render { path: "<full path>", destination: "<scratch-dir>" }
→ respond to prompts via `respond` until complete
```

Afterward, edit the generated `archetype.lua`, templates, and README in
place — do **not** regenerate from the starter once you've begun
authoring, or you'll clobber your work.

If the starters diverge from what new archetypes actually need (e.g.,
missing a common file, wrong author default, outdated manifest), open
an improvement on the starter rather than working around it in the
generated artifact.

## Common Development Patterns

- **Adding a new Rhai function**: Add to appropriate module in `archetect-core/src/script/rhai/modules/`, register in the module's `register()` function
- **Adding a template filter**: Register in the MiniJinja environment setup in `archetect-core/src/archetype/`
- **Adding a CLI subcommand**: Add clap variant in `archetect-bin/src/main.rs`, implement handler in `archetect-bin/src/subcommands/`
- **Adding a prompt type**: Implement in `archetect-core/src/script/rhai/modules/prompt_module/`, add message types in `archetect-api/src/commands/`
- **Modifying vendored crates** (`archetect-templating`, `archetect-terminal-prompts`): These are forks — changes stay local, no upstream sync expected

## Backwards Compatibility

There is an established catalog of archetypes used in production at the maintainer's company. **Backwards compatibility of the archetype syntax (Rhai scripting API, `archetype.yaml` manifest format) and configuration language (`archetect.yaml`) is critical.** Changes to these surfaces must not break existing archetypes or user configs. If a breaking change is truly necessary, it requires careful migration planning.

## Gotchas

- `archetect-templating` and `archetect-terminal-prompts` are **vendored forks**, not upstream dependencies. Edit them directly.
- The `build.rs` in `archetect-core` compiles `specs/archetect.proto` — if you modify the proto, regeneration happens automatically on build.
- `SystemLayout` has two implementations: `XdgSystemLayout` (production, XDG paths on Unix-likes including macOS, native on Windows) and `RootedSystemLayout` (custom root, used for tests via `RootedSystemLayout::temp()` and `RootedSystemLayout::new()`). The trait exposes `etc_dir`, `etc_d_dir`, `cache_dir`, and `data_dir`.
- v3 paths (XDG): config `~/.config/archetect/`, cache `~/.cache/archetect/`, data `~/.local/share/archetect/`. Lua IDE annotations live in `data_dir/lua/annotations`. v2 still uses `~/.archetect/` and is unaffected — both can coexist.
- The `feature/client-server` branch is stale. The gRPC client/server architecture was removed in favor of direct CLI invocation on main.
