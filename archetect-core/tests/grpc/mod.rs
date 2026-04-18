//! Integration tests exercising the real gRPC transport.
//!
//! These spawn an `ArchetectServer` on an ephemeral port, connect an
//! `ArchetectServiceClient` to it, and drive the bidirectional streaming API
//! message-by-message. Unlike the other integration tests (which use
//! `SyncClientIoHandle`), these validate the proto conversions, the async
//! bridge, and the lifecycle signals (`Initialize`, `Ack`, `CompleteSuccess`)
//! end-to-end.

mod harness;
mod basic_tests;
mod tls_tests;
