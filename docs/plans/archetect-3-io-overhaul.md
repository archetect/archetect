# Archetect 3: IO Protocol Overhaul

## Motivation

Archetect v2's `IoDriver` trait only covers prompts and logging. File writes go through direct `std::fs` calls in `archetype.rs`, making the core architecturally incompatible with remote/server operation. This blocks the planned CodegenExtension for COS.

A major version boundary gives us the opportunity to:
- Overhaul the IO protocol to be transport-agnostic (files, prompts, logging all flow through one channel)
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

The `feature/client-server` branch (2024-07-08) contains a working proof-of-concept with the right architectural ideas:

**Protocol Enrichment:**
- Asymmetric traits: `ScriptIoHandle` (core→client) + `ClientIoHandle` (client→core)
- `ScriptMessage` adds: `WriteFile(WriteFileInfo)`, `WriteDirectory(WriteDirectoryInfo)`, `CompleteSuccess`, `CompleteError`
- `ClientMessage` adds: `Ack` (write confirmations), `Initialize { answers_yaml, switches, use_defaults, use_defaults_all, destination }`
- `WriteFileInfo`: destination + contents as `Vec<u8>` + `ExistingFilePolicy` (Overwrite/Preserve/Prompt)
- Fallible send/receive (`Option<>` returns instead of panics)
- `DynClone` on traits for runtime polymorphism

**File Writing Through IO Channel:**
- `archetype.rs` sends WriteFile/WriteDirectory through the IO handle, waits for Ack
- Client side handles actual filesystem operations in `write_file_handler.rs` / `write_directory_handler.rs`
- Overwrite prompts live in the write handler (where user interaction belongs)

**gRPC Transport (reference implementation):**
- Bidirectional streaming proto: `StreamingApi(stream ClientMessage) returns (stream ScriptMessage)`
- tonic-based server/client with async IO bridge
- ~80% complete, functional but needs production hardening (panics in hot paths, no timeouts, no tests)

## Proposed Architecture for v3

### `archetect-api` — Enriched Protocol

- Rename: `CommandRequest` → `ScriptMessage`, `CommandResponse` → `ClientMessage`
- Rename: `IoDriver` → `ScriptIoHandle`, add `ClientIoHandle`
- Add: WriteFile, WriteDirectory, CompleteSuccess, CompleteError to ScriptMessage
- Add: Ack, Initialize to ClientMessage
- Fallible send/receive throughout (no panics)
- DynClone on traits
- Clean public API designed for external library consumption

### `archetect-core` — Transport-Agnostic Rendering

- File writes routed through IO channel (no direct std::fs in rendering path)
- `Archetect::request()` returns `Result<(), IoError>`, `Archetect::receive()` returns `Result<ClientMessage, IoError>`
- Proper error types replacing string-based errors
- Core becomes usable as a library by external products

### `archetect-terminal-io` — CLI Frontend

- `TerminalClient` with dedicated write handlers (ported from feature branch)
- `SyncIoDriver` split pattern: script side + client side connected by channels
- Handles WriteFile/WriteDirectory locally, Complete signals for lifecycle

### Separate Repo: CodegenExtension (closed source)

- Depends on `archetect-core` + `archetect-api` as library
- Implements `ScriptIoHandle` for network transport (gRPC, WebSocket, or COS-native)
- Server hosting, multi-tenant session management
- Database-backed catalog search
- Agent prompt routing for COS
- Authentication / authorization

## Error Handling Overhaul Opportunities

- Replace `.expect()` / `.unwrap()` in IO paths with proper error propagation
- Structured error types for `IoError`, `RenderError`, `ScriptError` instead of string wrapping
- `ArchetectIoDriverError` variants: `ScriptChannelClosed`, `ClientDisconnected`, `ClientError { message }`
- Audit all `anyhow` usage for cases that should have typed errors

## Backwards Compatibility

- **Archetype scripts**: Must remain compatible. Protocol changes are below the Rhai scripting layer.
- **User config**: Must remain compatible. Configuration format unchanged.
- **Rust API**: Breaking changes expected and acceptable at a major version boundary.
- **CLI behavior**: Identical end-user experience.

## Key Files (for reference)

Feature branch files to port/reference:
- `archetect-api/src/io_driver.rs` — ScriptIoHandle, ClientIoHandle, SyncIoDriver
- `archetect-api/src/commands.rs` — ScriptMessage, ClientMessage
- `archetect-api/src/commands/write_file_info.rs` — WriteFileInfo, ExistingFilePolicy
- `archetect-api/src/commands/write_directory_info.rs` — WriteDirectoryInfo
- `archetect-core/src/archetype/archetype.rs` — IO-channel-based file writing
- `archetect-terminal-io/src/` — TerminalClient, write_file_handler, write_directory_handler
- `archetect-core/src/io/async_impl.rs` — AsyncScriptIoHandle (reference for CodegenExtension)
- `archetect-core/specs/archetect.proto` — proto definition (reference for CodegenExtension)
