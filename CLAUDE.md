# CLAUDE.md

## Project Overview

**This is the Archetect 3 repository** — a major version evolution of Archetect, the code-centric, language-agnostic code generator written in Rust.

- **v3 repo**: https://github.com/archetect/archetect-3
- **v2 repo** (stable, production): https://github.com/archetect/archetect
- **Docs** (v2): https://archetect.github.io
- **GitHub Org**: https://github.com/archetect
- **License**: MIT

The workspace is at **3.4.0**. v3 is its own codebase now, not a layer on top of v2.

## v3 Initiatives

Plans live in `docs/plans/`; specs in `docs/specs/`. The major arcs:

1. **[IO Protocol Overhaul](docs/plans/archetect-3-io-overhaul.md)** — Route file writes through the IO channel rather than direct `std::fs`. `ScriptIoHandle`/`ClientIoHandle` traits, fallible error handling, WriteFile/WriteDirectory/Complete/Ack/Initialize messages.

2. **[Lua Scripting Engine](docs/plans/archetect-3-lua-scripting-engine.md)** — Lua (via mlua) as **the** scripting engine, a clean-slate redesign: `Context` object, dedicated prompt methods, simplified case system, namespaced modules, LuaLS annotations for IDE support. **Rhai has been removed entirely** — there is no `script/rhai/` and no rhai dependency.

3. **[Dynamic Interface](docs/plans/dynamic-interface.md)** — Derive an archetype's interface by probing its script, so a UI can render a form without hand-maintaining one. Backs `archetect interface`, the MCP `describe` tool, and the gRPC `DescribeArchetype` RPC.

4. **[Federated Catalog](docs/plans/federated-catalog.md)** — Catalog browse/search over the wire, so a client can navigate a server's catalog.

5. **[Model-Driven Generation](docs/plans/archetect-3-model-driven-generation.md)** — AML: generate whole architectures from a domain model (`archetect-aml`).

## Related Projects

- **v2 codebase**: the `2.x` bookmark — stable production CLI, do not mix with v3 work
- **Production archetypes**: `/Users/jimmie/work/p6m-archetypes` — the real-world catalog v3 must serve
- **COS / Onyx**: `/Users/jimmie/personal/jimmiebfulton/onyx` — the CodegenExtension target platform
- **Archetect Studio**: `/Users/jimmie/personal/archetect-swift` — SwiftUI client, first consumer of the client/server surface

## Workspace Structure

Cargo workspace with 10 members:

```
archetect-bin (CLI entry point)
├── archetect-core (business logic, Lua scripting, ATL rendering, gRPC client + server)
│   ├── archetect-api (ScriptIoHandle/ClientIoHandle traits, command + envelope types)
│   ├── archetect-git-cache (git source caching)
│   ├── archetect-inflections (case conversions, pluralization)
│   ├── archetect-terminal-io (terminal IoDriver impl + terminal client)
│   └── archetect-validations (validation error types)
├── archetect-aml (AML: model-driven architecture generation)
└── archetect-mcp (MCP server: learn/introspect/describe/render over stdio)
xtask (build/install automation)
```

### Crate Purposes

| Crate | Purpose |
|-------|---------|
| `archetect-bin` | CLI (clap), configuration loading (figment), subcommand dispatch |
| `archetect-core` | Archetype/catalog loading, Lua engine, ATL templating, source/cache, gRPC client + server |
| `archetect-api` | `ScriptIoHandle`/`ClientIoHandle` traits, prompt/write commands, `PromptEnvelope` |
| `archetect-aml` | Domain-model expansion for architecture generation |
| `archetect-mcp` | MCP server surface — session, prompt envelopes, tool dispatch |
| `archetect-git-cache` | Git source fetching, caching, locking |
| `archetect-terminal-io` | `TerminalIoDriver` + `TerminalClient` — the reference IO client |
| `archetect-inflections` | String transforms: camelCase, snake_case, plural/singular, etc. |
| `archetect-validations` | Validation rules and error types |
| `xtask` | `cargo xtask install` with optional `--static-openssl` |

## Build & Development Commands

```bash
cargo build
cargo test                          # entire workspace
cargo test -p archetect-core        # one crate
cargo xtask install                 # install CLI locally
cargo run -p archetect-bin -- <args>
cargo clippy --all-targets --all-features
cargo fmt
```

`build.rs` in `archetect-core` compiles `specs/archetect.proto`. A vendored `protoc` is used
automatically, so no system install is required (an explicit `PROTOC` env var still wins).

## Architecture Deep Dive

### Core Flow: Rendering an Archetype

1. CLI parses args → loads Configuration (YAML + CLI overrides via figment)
2. `Source` resolves the archetype origin (local path or Git URL with caching)
3. `Archetype` loads `archetype.yaml` and validates requirements
4. The Lua engine executes `archetype.lua` (or the manifest-specified script)
5. Script prompts via `Context` methods → `ScriptMessage` over `ScriptIoHandle` → client responds
6. Script calls `directory.render()` / `file.render()` / `template.render()` → ATL processes templates
7. Rendered files go out as `WriteFile`/`WriteDirectory` messages — **the client writes them**

### Key Source Files

| File | What it does |
|------|-------------|
| `archetect-bin/src/main.rs` | CLI entry, arg parsing, subcommand dispatch |
| `archetect-core/src/archetect/archetect.rs` | `Archetect` — main orchestrator, builder pattern |
| `archetect-core/src/archetype/archetype.rs` | `Archetype` — loading, rendering, script execution |
| `archetect-core/src/script/lua/` | Lua engine setup, `Context`, cases, module registration |
| `archetect-core/src/templating/atl/` | ATL — the Jinja-shaped template engine |
| `archetect-core/src/interface/` | Interface probing (`DerivedInterface`, `ProbeOptions`) |
| `archetect-core/src/server/` | gRPC service (`ArchetectServiceCore`) and server lifecycle |
| `archetect-core/src/client/` | gRPC client — drives `StreamingApi`, materializes writes |
| `archetect-core/src/catalog/` | Catalog loading, indexing, dispatch |
| `archetect-core/src/system.rs` | `SystemLayout` — filesystem layout abstraction |
| `archetect-api/src/prompt_envelope.rs` | `PromptEnvelope` — one prompt as every client sees it |
| `archetect-core/specs/archetect.proto` | The wire contract for client/server |

### Scripting API — ask the binary, don't grep

The Lua surface changes faster than any table in this file. **Never guess an API shape:**

```bash
archetect learn                 # topic catalog (authoring, templates, catalogs, mcp, model…)
archetect learn <topic>         # one screen, computed for THIS environment
archetect introspect <filter>   # every Context method, module function, prompt option
archetect eval '<lua>'          # probe live behavior, headless
```

Modules are `require`d under `archetect.*` (`archetect.shell`, `archetect.git`,
`archetect.github`, `archetect.archive`, `archetect.model`). Rendering verbs
(`directory`, `file`, `template`, `catalog`) and `Context` are available directly.

### Template System (ATL)

- Jinja-shaped syntax: `{{ variable }}`, `{% if %}`, `{% for %}` — Lua underneath
- Inflection filters: `{{ name | snake_case }}`, `{{ name | pluralize }}`
- Templates live in the archetype's `contents/` (directory render) or `templates/` (single-file)
- Directory and file names are parameterized: `{{ artifact_id }}/src/main.rs`

### Configuration

XDG paths on Unix-likes (including macOS), native on Windows:

- config `~/.config/archetect/`, cache `~/.cache/archetect/`, data `~/.local/share/archetect/`
- Lua IDE annotations live in `data_dir/lua/annotations`
- v2 still uses `~/.archetect/` and is unaffected — both coexist

Merge order: built-in defaults → user config → `etc.d/*.yaml` drop-ins → project-local →
`--config-file` → CLI flags. Key sections: `actions`, `catalog`, `server`, `client`, `offline`,
`headless`, `answers`, `switches`, `security`, `locals`, `updates`.

### IO Driver Architecture

All script↔user communication flows through `ScriptIoHandle` (trait in `archetect-api`):

- Script sends `ScriptMessage` (prompts, file writes, logs)
- Client responds with `ClientMessage` (string, int, bool, array, ack, abort)
- `TerminalIoDriver` implements this for CLI interaction; the MCP session and the gRPC
  server are alternative drivers over the same protocol

**`WriteFile`/`WriteDirectory` are script→client messages: the CLIENT owns the filesystem.**
In server mode the server streams bytes and never writes the tree itself — which is why a
relative `--destination` resolves against the *client's* cwd.

`archetect.archive` follows from that: it packages the render's **write journal**
(`Archetect::archive_entries_under`), not the disk, and emits the result as an ordinary
`WriteFile`. So archives work identically local and remote, and a client needs no archiving
capability of its own. Files produced by a shell-out are invisible to it — they never
crossed the channel.

**Still bypassing the channel:** `archetect.git` and `archetect.shell` operate directly on
`render_context.destination()`, which in server mode is never populated. They are effectively
CLI-only today. If you make them work over the wire, prove it in
`proofs/server/materialization_test.lua` alongside the archive proofs.

### Capabilities

Effects that reach *outside* the destination are gated. An archetype declares what it needs
in the manifest (`requires.capabilities`, currently just `publish`); the caller grants what
it allows. Local renders are unrestricted — the user is the trust boundary. A connected
session is **default-deny**, granting only what `--allow` names, and refuses before rendering
so a refusal never leaves a half-written tree. `Archetect::grants()` is the single predicate;
`restrict_capabilities` is set-once via `OnceLock`, so a session cannot widen its own grants.

This is what makes it safe to host an open catalog: without it, any archetype on the server
could reach GitHub with the host's token.

### Client / Server

`archetect server` serves gRPC (tonic, with reflection + health); `archetect connect
<endpoint> [path]` is the reference client. The contract is
`archetect-core/specs/archetect.proto`:

| RPC | Purpose |
|---|---|
| `StreamingApi` (bidi) | the interactive render session — prompts and writes both cross here |
| `BrowseCatalog` / `SearchCatalog` | catalog navigation from the server's startup index |
| `DescribeArchetype` | derived interface as JSON — lets a UI render a form up front |

TLS is configurable on both ends (`--tls-cert`/`--tls-key`, `--tls-ca`, mutual TLS).
`Initialize` selects a `catalog_path` and carries the capabilities the client grants; there is
no free-form source over the wire. Paths resolve through the same walker `DescribeArchetype`
uses (`dispatch::walk_path` + `render_leaf`, so entry answers/switches apply); with no path,
only an UNAMBIGUOUS default renders — the server's startup action (`archetect server <action>`,
validated at startup, fail-fast), a "default" entry, or a single-leaf catalog — never the
first entry by declaration order (`proofs/server/action_resolution_test.lua`).
`CompleteSuccess` carries an artifact manifest — what the
render produced (archives, published repos) with script-derived names a caller could not
otherwise guess. `archetect connect` prints it; that is how Ybor Studio learns which zip to
offer for download.

### Source & Caching

- Git sources cached under the XDG cache dir; pull timestamps tracked via git notes
- Supports branch/tag refs; local overrides via the `locals` config section
- `archetect cache clear|pull|manage`

## Testing

Two layers and four tiers. The layers answer different questions (does the shipped binary
behave? do the pieces?); the tiers decide how much of the bar you are asking for right now.

### prova — black-box acceptance proofs

**Prova proves what archetect ships.** `.prova.toml` sets `proofs = ["proofs"]` and declares
the lanes below.

```bash
prova                                   # everything
prova proofs/server/materialization_test.lua
prova --node "a relative destination resolves on the CLIENT, not the server"
prova --last-failed                     # re-run only what's red
prova tests --promises                  # the open executable backlog (nothing runs)
prova owed                              # what is promised and not yet kept
prova learn <topic>                     # promises, authoring, fixtures, drivers, running…
```

Working practice (PDD):

1. Write the proof first. Red is correct at that stage.
2. Implement until green. **Never weaken a proof to pass it** — fix the system, or
   renegotiate the bar.
3. Commit suite + implementation together as a proof-carrying change.

**Promises are the executable backlog.** A contract you can state but are not implementing
now gets authored flagged `{ promises = "reason" }` — the reason is mandatory and carries the
*why* while the proof is red. Open promises keep CI green (outcome `PROMISED`) and are listed
by `prova owed` / `prova tests --promises`. When a promise's body starts passing it **fails**,
demanding graduation: convert `promises = "..."` to `proves = "..."` (preferred — the design
story stays next to the assertions) in the same commit as the implementation. `proves` is
runtime-inert and can be retrofitted to any existing proof.

(`spec = "..."` was prova's older spelling and is refused outright since 0.18 — see
`prova learn promises`.)

### The four tiers

They answer four different questions, which is why they are four profiles and not one dial:

| Command | Question | Cost |
|---|---|---|
| `prova` | is this correct? | seconds — the inner loop |
| `prova run ut` | do the unit tests hold? | + one `cargo nextest` |
| `prova run all` | am I checking in DEBT? | ~15s — before commit |
| `prova run release` | is the whole bar met? | ~35s — before cutting a version |

`prova run --list` prints them with descriptions. The middle one is the one worth explaining:
green proofs say the behavior is right; they say nothing about a `.unwrap()` added on the way
there, a file grown past 1,500 lines, or a clippy finding that rode along. `all` is where that
difference gets caught — by **ratchets** against `.prova/baselines/quality.json`, so the bar can
only move one way without a deliberate, reviewable edit.

- **`ut`** (`proofs/ut/`) — `cargo nextest` conducted ONCE, every case adopted into the account
  (`junit.verify`), then case-granular readers bind specific contracts to named unit tests. One
  `prova` is the whole quality account, not a partial one.
- **`quality`** (`proofs/quality/`) — clippy findings, `.unwrap()`/`.expect()` in shipped code,
  and oversized files, each ratcheted. Paid some down? `prova run quality --update-baseline`
  tightens the floor (it refuses to loosen). The file-size gate carries no switch, so it runs in
  the default loop too.
- **`coverage`** (`proofs/coverage/`) — three numbers from one conduct: unit (`cargo llvm-cov
  nextest`), black-box (the whole suite driving an **instrumented** archetect), and the merge.
  The delta is the signal — the log names files proven black-box but naked at the unit layer.

CI: `build.yml` runs `prova -s ut` on every branch push; `quality.yml` runs `prova run release`
nightly and on dispatch. Both install a pinned, checksummed prova.

**Bank baselines on macOS, not in CI.** Every ratcheted metric reads slightly looser on
macOS/aarch64 than on the Linux runner (measured: 136 clippy findings there against the 138
banked here; coverage within 0.05pp), so a macOS-banked floor passes both and a Linux-banked
one turns local runs red for a platform difference nobody can act on.

**Report custody outruns the release.** `report.publish` exists in a dev build of prova but not
in v0.24.0, which is what CI installs — so all three conducts publish through
`require("custody")`, which takes custody where the runner supports it and prints the summary
where it cannot. Delete the shim once the pinned prova has `report`.

### Reports

A conduct's artifact is kept, not discarded with `target/`: `prova reports` lists what exists
(`clippy`, `unit-cases`, `coverage`), `prova reports coverage --kind html` prints one path.

Conventions in this repo:

- **`require("subject").bin`** is the binary under proof — one `cargo build -p archetect` per
  RUN, shared by every suite, and overridable via `ARCHETECT_SUBJECT_BIN` (which is how the
  coverage lane points the suite at an instrumented build). Never re-declare it per file.
- Servers are spawned with `shell.spawn` on a `net.free_port()`, then `grpc.wait_for`.
- **prova's gRPC driver is unary-only** — it cannot drive the bidi `StreamingApi`. Prove
  streaming behavior through `archetect connect` via `shell.run` argv instead. That is the
  real client, so it stays genuinely black-box.
- Give the server its own cwd when the proof needs to distinguish which side wrote a file.

### cargo test — unit and integration

Reachable directly, and also **conducted by prova** as a deputy (`prova run ut`) so its verdicts
land in the same account as the proofs.

```bash
cargo test                          # all workspace tests
cargo nextest run --workspace       # what the ut lane conducts
cargo test -p archetect-core        # core only
cargo test -p archetect-inflections # case conversions
```

- **`archetect-core/tests/`** — integration tests via `TestHarness` (`test_utils.rs`), covering
  `prompts/`, `cases/`, `catalog/`, `context/`, `errors/`, `git/`, `github/`, `grpc/`, `headless/`
- **Inline `#[cfg(test)]`** modules throughout source files

`TestHarness` spawns a render on a separate thread, uses `SyncIoDriver` for programmatic
prompt/response, and checks `render_succeeded()`.

### Testing with real archetypes

```bash
cargo run -p archetect-bin -- render <source> /tmp/test-output -D    # -D = use all defaults
archetect interface <source>                                          # derive the prompt contract
archetect interface <source> --answers-template                       # emit a fill-in answers file
```

## Archetype Anatomy

```
my-archetype/
├── archetype.yaml          # Manifest: description, authors, requires, scripting, templating
├── archetype.lua           # Main Lua script (or path specified in manifest)
├── lua/                    # Additional Lua modules
├── contents/               # Template directory (rendered via directory.render())
│   └── {{ project_name }}/
│       └── src/
│           └── main.rs
└── templates/              # Individual templates (rendered via file.render())
```

```yaml
description: "My Archetype"
authors: ["Author"]
languages: ["Rust"]
frameworks: ["Actix"]
tags: ["web", "api"]
requires:
  archetect: "3.0.0"
scripting:
  main: "archetype.lua"
  modules: "lua"
templating:
  content_directory: "contents"
  templates_directory: "templates"
  undefined_behavior: "strict"   # strict | lenient | chainable
catalog:
  child-name:
    source: "https://github.com/org/child-archetype.git"
```

## CI/CD

GitHub Actions in `.github/workflows/`:
- **`build.yml`** — all branch pushes: `cargo build` + `cargo test` on Ubuntu 24.04
- **`release.yml`** — tag pushes: cross-platform builds (Linux x64, macOS aarch64, Windows x64),
  GitHub release with archives and checksums

## Version Control

This repository uses **Jujutsu (jj)**. Use `jj`, never `git`.

```bash
jj st --no-pager          # status
jj log --no-pager         # history
jj diff --no-pager        # working-copy diff
jj commit -m "..."        # seal completed work AND start a fresh empty @
jj new -m "..."           # new empty change on top
jj bookmark list          # bookmarks (labels, not branches)
```

`jj commit` — not `jj describe` — is how finished work is sealed; describe leaves the change
as the working copy, so the next edit silently piles into it. This repo is outside `~/work/`,
so **do not sign**. Never push, move bookmarks, or squash without being asked.

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
| Archetype | `archetect/common/starters/archetype-starter` |
| Catalog | `archetect/common/starters/catalog-starter` |
| Library | `archetect/common/starters/library-starter` |

(There is no component starter in the catalog — `library-starter` covers prompts, content,
context return, and lib/include exports. Verify with `catalog_browse` before assuming a path.)

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
- `learn { topic? }` / `introspect { filter? }` — the knowledge surface
- `catalog_search { query }` — discover archetypes by keyword (AND terms)
- `catalog_browse { path? }` — walk the catalog tree
- `describe { source | path, explore? }` — derive the prompt contract BEFORE rendering
- `catalog_render { path, destination, answers?, switches?, use_defaults_all? }` — render by catalog path
- `render { source, destination, ... }` — render from a URL or local path
- `respond { value }` / `cancel` — answer or abort an interactive prompt

Shell-exec is forbidden in MCP mode by design; a render needing `--allow-exec` is a CLI move.
Users working interactively at the CLI still use `archetect render` directly.

### Typical agent flow

```
catalog_search { query: "archetype starter" }
→ discover archetect/common/starters/archetype-starter
describe { path: "<full path>" }
→ prepare answers from the derived interface
catalog_render { path: "<full path>", destination: "<scratch-dir>", answers: {...} }
```

Afterward, edit the generated `archetype.lua`, templates, and README in
place — do **not** regenerate from the starter once you've begun
authoring, or you'll clobber your work.

If the starters diverge from what new archetypes actually need (e.g.,
missing a common file, wrong author default, outdated manifest), open
an improvement on the starter rather than working around it in the
generated artifact.

## Common Development Patterns

- **Adding a Lua function**: add to the appropriate module in
  `archetect-core/src/script/lua/`, register it in `require_modules.rs` (or `modules.rs` for
  globals), and add the LuaLS annotation so `introspect` and IDEs both see it
- **Adding a template filter**: register in the ATL environment setup under
  `archetect-core/src/templating/atl/`
- **Adding a CLI subcommand**: add the clap command in `archetect-bin/src/cli.rs`, implement
  the handler in `archetect-bin/src/subcommands/`, dispatch from `main.rs`
- **Adding a prompt type**: implement in the Lua prompt surface, add message types in
  `archetect-api/src/commands/`, extend `PromptEnvelope`, and add the proto variant
- **Changing the wire protocol**: edit `archetect-core/specs/archetect.proto`; `build.rs`
  regenerates on build. Update `archetect-core/src/proto/conversions.rs` in the same change.

## Backwards Compatibility

There is an established catalog of archetypes used in production. **Backwards compatibility of
the archetype syntax (Lua scripting API, `archetype.yaml` manifest format) and configuration
language is critical.** Changes to these surfaces must not break existing archetypes or user
configs. If a breaking change is truly necessary, it requires careful migration planning.

Because the scripting API is a public contract, prefer adding to it over reshaping it, and
prove additions with a proof rather than a unit test where the behavior is user-visible.

## Gotchas

- **CLI arg lookups must be fallible across subcommands.** `clap`'s `get_one` *panics* on an
  arg id the subcommand never declared. `archetect connect` shipped broken for exactly this
  reason. Use `try_get_one` in any helper shared between subcommands.
- **A remote render must report failure.** The client cannot see the server's logs — a
  server-side render error has to become a `CompleteError` and a non-zero client exit, or a
  broken generation looks identical to a successful one.
- `SystemLayout` has two implementations: `XdgSystemLayout` (production) and
  `RootedSystemLayout` (custom root, for tests via `::temp()` / `::new()`). The trait exposes
  `etc_dir`, `etc_d_dir`, `cache_dir`, `data_dir`.
- ATL is a first-party engine under `archetect-core/src/templating/atl/`, no longer a vendored
  MiniJinja crate. The old `archetect-templating` and `archetect-terminal-prompts` crates are
  gone.
- The catalog `server:` entry key lets a catalog entry delegate to a remote Archetect server.
