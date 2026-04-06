# Archetect 3: IO Protocol Overhaul

## Motivation

Archetect v2's `IoDriver` trait only covers prompts and logging. File writes go through direct `std::fs` calls in `archetype.rs`, making the core architecturally incompatible with remote/server operation. This blocks both the planned CodegenExtension for COS and the ability for the CLI itself to operate as a client against a remote Archetect server.

A major version boundary gives us the opportunity to:
- Overhaul the IO protocol to be transport-agnostic (files, prompts, logging all flow through one channel)
- Add first-class client/server capability to Archetect itself (gRPC bidirectional streaming)
- Restructure error handling (replace panics/unwraps with proper Result types)
- Clean up the Rust public API of `archetect-api` and `archetect-core` for library consumption
- Incorporate lessons from the `feature/client-server` branch proof-of-concept

## Current State (v2.1.0 on main)

**IO Protocol (`archetect-api`):**
- Single `IoDriver` trait: `send(CommandRequest)` + `receive() -> CommandResponse`
- `CommandRequest`: 7 prompt types, 5 log levels, Print, Display (14 variants)
- `CommandResponse`: String, Integer, Boolean, Array, None, Error, Abort (7 variants)
- No file write commands, no completion signals, no initialization message
- `send()` is fire-and-forget, `receive()` panics on channel errors

**File Writing (`archetect-core`):**
- `archetype.rs` calls `std::fs::create_dir_all`, `File::create`, `fs::copy` directly
- Overwrite prompts use `archetect_inquire::Confirm` inline, bypassing IoDriver
- File I/O is invisible to the IO layer — cannot be intercepted, remoted, or observed

## Prior Art: `feature/client-server` Branch

The `feature/client-server` branch (bookmark in this repo, fetched from v2) contains a working proof-of-concept with the right architectural ideas. This branch is the primary reference for v3 implementation — we adopt its design decisions and improve on its gaps.

**Protocol Enrichment:**
- Asymmetric traits: `ScriptIoHandle` (core->client) + `ClientIoHandle` (client->core)
- `ScriptMessage` adds: `WriteFile(WriteFileInfo)`, `WriteDirectory(WriteDirectoryInfo)`, `CompleteSuccess`, `CompleteError`
- `ClientMessage` adds: `Ack` (write confirmations), `Initialize { answers_yaml, switches, use_defaults, use_defaults_all, destination }`
- `WriteFileInfo`: destination + contents as `Vec<u8>` + `ExistingFilePolicy` (Overwrite/Preserve/Prompt)
- Fallible send/receive (`Option<>` returns instead of panics)
- `DynClone` on traits for runtime polymorphism

**File Writing Through IO Channel:**
- `archetype.rs` sends WriteFile/WriteDirectory through the IO handle, waits for Ack
- Client side handles actual filesystem operations in `write_file_handler.rs` / `write_directory_handler.rs`
- Overwrite prompts live in the write handler (where user interaction belongs)

**gRPC Transport:**
- Bidirectional streaming proto: `StreamingApi(stream ClientMessage) returns (stream ScriptMessage)`
- tonic-based server/client with async IO bridge
- `AsyncScriptIoHandle` / `AsyncClientIoHandle` bridging async gRPC to synchronous Rhai execution via `blocking_send`/`blocking_recv`
- Server builder pattern: `ArchetectServer::builder(core).build().await?`
- Client connects via `archetect connect --endpoint <host:port>`
- ~80% complete — functional but needs production hardening

**Known Gaps in Feature Branch:**
- `.expect()` / `.unwrap()` in RPC handler hot paths
- Hardcoded 5-second timeout with no configurability
- No keepalive/heartbeat for stale connection detection
- `Mutex<Receiver>` in async code (potential contention)
- No tests
- Proto <-> Rust message conversions need cleanup

## Proposed Architecture for v3

### Design Principle: Archetect Is Both Server and Client

The gRPC server is not specific to the CodegenExtension — it is a first-class Archetect capability. The same server can be accessed by:
- The Archetect CLI (`archetect connect`)
- The CodegenExtension (COS/Substrate)
- Future clients (IDE plugins, web UIs, CI/CD integrations)

This means hidden/private archetypes served from an Archetect server are accessible from any client. The CodegenExtension adds multi-tenancy, auth, and agent routing on top — it doesn't own the server.

```
                    ┌─────────────────────┐
                    │   Archetect Server   │
                    │  (archetect-core +   │
                    │   gRPC transport)    │
                    └────────┬────────────┘
                             │
              ┌──────────────┼──────────────┐
              │              │              │
         CLI Client    CodegenExt     Future clients
        (archetect-bin)  (closed src)   (IDE, CI, web)
```

### `archetect-api` — Enriched Protocol

- Rename: `CommandRequest` -> `ScriptMessage`, `CommandResponse` -> `ClientMessage`
- Rename: `IoDriver` -> `ScriptIoHandle`, add `ClientIoHandle`
- Add to `ScriptMessage`: `WriteFile`, `WriteDirectory`, `CompleteSuccess`, `CompleteError`
- Add to `ClientMessage`: `Ack`, `Initialize`
- `WriteFileInfo`: destination path, contents (`Vec<u8>`), `ExistingFilePolicy` enum
- `WriteDirectoryInfo`: path
- Fallible send/receive throughout — `send() -> Result<(), IoError>`, `receive() -> Result<ClientMessage, IoError>`
- `DynClone` on traits for runtime polymorphism
- `SyncIoDriver` split pattern: script side + client side connected by crossbeam/mpsc channels
- Clean public API designed for external library consumption

### `archetect-core` — Transport-Agnostic Rendering

- File writes routed through IO channel (no direct `std::fs` in rendering path)
- `Archetect::request()` / `Archetect::receive()` return `Result` types
- Proper error types: `IoError`, `RenderError`, `ScriptError` replacing string-based errors
- Async IO bridge: `AsyncScriptIoHandle` / `AsyncClientIoHandle` using tokio channels with `blocking_send`/`blocking_recv` to connect async gRPC with synchronous script execution
- gRPC server implementation: `ArchetectServer` with bidirectional streaming
- Core usable as a library by external products (CodegenExtension)

### `archetect-terminal-io` — Terminal Client

- `TerminalClient` with dedicated write handlers (file and directory)
- Implements `ClientIoHandle` for interactive terminal use
- Handles `WriteFile`/`WriteDirectory` locally (filesystem operations + overwrite prompts)
- Handles `CompleteSuccess`/`CompleteError` for lifecycle management

### `archetect-bin` — CLI with Local and Remote Modes

- **Local mode** (default): `archetect render <source> <dest>` — works as today, in-process
- **Server mode**: `archetect server` — starts gRPC server on configured port, serves archetypes
- **Client mode**: `archetect connect --endpoint <host:port> <args>` — connects to remote server, renders via gRPC
- Server configuration in `archetect.yaml` (host, port, TLS settings)

### Proto Definition

The gRPC service uses a single bidirectional streaming RPC (from the feature branch, with improvements):

```protobuf
service ArchetectService {
    rpc StreamingApi (stream ClientMessage) returns (stream ScriptMessage);
}
```

Key message types:
- `ScriptMessage`: prompts (7 types), logging (5 levels), print/display, `WriteFile`, `WriteDirectory`, `CompleteSuccess`, `CompleteError`
- `ClientMessage`: `Initialize`, response types (string, int, bool, array, none), `Error`, `Abort`, `Ack`
- `ExistingFilePolicy`: Preserve, Overwrite, Prompt

## Implementation Phases

### Phase 1: Protocol Foundation (`archetect-api`)

Enrich the protocol types and traits without changing behavior.

1. Rename `CommandRequest` -> `ScriptMessage`, `CommandResponse` -> `ClientMessage`
2. Rename `IoDriver` -> `ScriptIoHandle`, add `ClientIoHandle` trait
3. Add `WriteFile(WriteFileInfo)`, `WriteDirectory(WriteDirectoryInfo)`, `CompleteSuccess`, `CompleteError` to `ScriptMessage`
4. Add `Ack`, `Initialize` to `ClientMessage`
5. Make send/receive fallible (`Result` returns)
6. Add `DynClone` to traits
7. Update `SyncIoDriver` to implement split pattern (script handle + client handle)
8. Fix all callsites in `archetect-core`, `archetect-terminal-io`, `archetect-bin` to compile

**Exit criteria:** `cargo build && cargo test` pass. No behavior changes — existing prompts/logging work identically.

### Phase 2: File Writes Through IO Channel (`archetect-core` + `archetect-terminal-io`)

Route file operations through the protocol instead of direct `std::fs`.

1. Remove direct filesystem calls from `archetype.rs` rendering path
2. Send `WriteFile` / `WriteDirectory` messages through `ScriptIoHandle`
3. Wait for `Ack` response before continuing
4. Implement `write_file_handler` and `write_directory_handler` in `archetect-terminal-io`
5. Move overwrite prompts into write handlers (where user interaction belongs)
6. Send `CompleteSuccess` / `CompleteError` at archetype execution boundaries

**Exit criteria:** All file writes flow through IO channel. Existing archetypes produce identical output. Tests pass.

### Phase 3: Error Handling Overhaul

Replace panics and string errors with proper error types.

1. Define `IoError` enum: `ScriptChannelClosed`, `ClientDisconnected`, `ClientError { message }`, `Timeout`
2. Define `RenderError`, `ScriptError` with structured variants
3. Audit and replace `.unwrap()` / `.expect()` in IO paths
4. Audit `anyhow` usage for cases that should have typed errors
5. Ensure errors propagate cleanly from IO layer through script engine to CLI

**Exit criteria:** No panics in IO/rendering hot paths. Errors carry structured context.

### Phase 4: gRPC Server and Client

Add client/server capability using tonic.

1. Update `archetect.proto` (refine from feature branch, add versioning fields)
2. Implement proto <-> Rust message conversions (`From`/`Into` traits)
3. Port and improve `AsyncScriptIoHandle` / `AsyncClientIoHandle` (eliminate Mutex contention, add proper error handling)
4. Port and improve `ArchetectServer` (configurable timeouts, graceful shutdown, connection keepalive)
5. Port and improve client connection logic
6. Add `archetect server` subcommand
7. Add `archetect connect` subcommand
8. Server configuration in `archetect.yaml`
9. Integration tests: full round-trip render via gRPC (prompt, file write, completion)

**Exit criteria:** `archetect server` serves archetypes. `archetect connect` renders against a running server. Integration tests cover the critical path.

### Phase 5: Hardening

Production-readiness improvements.

1. TLS support for gRPC transport
2. Connection keepalive / heartbeat
3. Configurable timeouts
4. Health check endpoint (`grpc.health.v1`)
5. Graceful shutdown with in-flight request draining
6. Server-side logging and observability
7. Client reconnection / retry logic

## Error Handling Strategy

| Layer | Current | v3 Target |
|-------|---------|-----------|
| IO send/receive | Panics on channel error | `Result<(), IoError>` / `Result<T, IoError>` |
| File writing | `unwrap()` on fs ops | `WriteFile` through IO channel, `Ack` or error response |
| Script execution | String-wrapped errors | `ScriptError` with structured variants |
| Template rendering | Mixed anyhow/string | `RenderError` with source location context |
| gRPC transport | `.expect()` in handlers | Proper tonic `Status` mapping |

## Backwards Compatibility

- **Archetype scripts**: Must remain compatible. Protocol changes are below the Rhai scripting layer.
- **User config**: Must remain compatible. New server config fields are additive.
- **CLI behavior**: Local mode identical. New `server` and `connect` subcommands are additive.
- **Rust API**: Breaking changes expected and acceptable at a major version boundary.

## Key Reference Files

Feature branch files (`jj file show -r feature/client-server <path>`):
- `archetect-api/src/io_driver.rs` — ScriptIoHandle, ClientIoHandle, SyncIoDriver
- `archetect-api/src/commands.rs` — ScriptMessage, ClientMessage
- `archetect-api/src/commands/write_file_info.rs` — WriteFileInfo, ExistingFilePolicy
- `archetect-api/src/commands/write_directory_info.rs` — WriteDirectoryInfo
- `archetect-core/src/archetype/archetype.rs` — IO-channel-based file writing
- `archetect-core/src/io/async_impl.rs` — AsyncScriptIoHandle/AsyncClientIoHandle
- `archetect-core/src/server/` — ArchetectServer, gRPC service core
- `archetect-core/src/client/client.rs` — Client connection logic
- `archetect-terminal-io/src/` — TerminalClient, write_file_handler, write_directory_handler
- `archetect-core/specs/archetect.proto` — proto definition
