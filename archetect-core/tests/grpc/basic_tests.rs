use std::time::Duration;

use tokio_stream::StreamExt;

use archetect_core::catalog::catalog_indexer::CatalogIndexer;
use archetect_core::client::ClientOptions;
use archetect_core::configuration::Configuration;
use archetect_core::manifest::{CatalogEntry, CatalogEntryServer};
use archetect_core::proto::grpc::script_message::Message as SMessage;
use archetect_core::Archetect;

use super::harness::{build_catalog, build_nested_entry, msg, TestServer};
use linked_hash_map::LinkedHashMap;

/// Pull the next `ScriptMessage` off the stream, panicking on timeout or
/// error. Keeps the test flow readable without boilerplate at every await.
async fn next(stream: &mut tonic::Streaming<archetect_core::proto::grpc::ScriptMessage>) -> SMessage {
    let msg = tokio::time::timeout(Duration::from_secs(10), stream.next())
        .await
        .expect("timed out waiting for ScriptMessage")
        .expect("stream ended prematurely")
        .expect("gRPC error receiving ScriptMessage");
    msg.message.expect("ScriptMessage oneof was None")
}

#[tokio::test]
async fn grpc_basic_roundtrip() {
    let mut server = TestServer::start("grpc_basic").await.expect("server up");

    let tmp = tempfile::tempdir().expect("tempdir");
    let destination = tmp.path().to_string_lossy().to_string();

    let (tx, mut stream) = server.open_stream().await.expect("open stream");

    tx.send(msg::initialize(destination.clone(), String::new()))
        .await
        .expect("initialize send");

    // Expect: PromptForText("Name:") — respond with String
    match next(&mut stream).await {
        SMessage::PromptForText(p) => {
            assert_eq!(p.message, "Name:");
            tx.send(msg::string("world".to_string())).await.expect("string resp");
        }
        other => panic!("expected PromptForText, got {:?}", other),
    }

    // Expect: LogInfo("rendering for world")
    let mut saw_log = false;
    let mut saw_write_dir = false;
    let mut saw_write_file = false;

    loop {
        match next(&mut stream).await {
            SMessage::LogInfo(m) => {
                if m.contains("world") {
                    saw_log = true;
                }
            }
            SMessage::WriteDirectory(_) => {
                saw_write_dir = true;
                tx.send(msg::ack()).await.expect("ack dir");
            }
            SMessage::WriteFile(wf) => {
                saw_write_file = true;
                assert!(
                    wf.destination.ends_with("greeting.txt"),
                    "unexpected write destination: {}",
                    wf.destination
                );
                let body = String::from_utf8_lossy(&wf.contents);
                assert!(
                    body.contains("Hello, world!"),
                    "unexpected file contents: {}",
                    body
                );
                tx.send(msg::ack()).await.expect("ack file");
            }
            SMessage::CompleteSuccess(_) => {
                break;
            }
            SMessage::CompleteError(err) => {
                panic!("render failed: {:?}", err);
            }
            _ => {
                // Display, Print, LogDebug, etc. — not asserted on.
            }
        }
    }

    assert!(saw_log, "expected a LogInfo mentioning 'world'");
    assert!(saw_write_dir, "expected at least one WriteDirectory");
    assert!(saw_write_file, "expected WriteFile for greeting.txt");
}

/// Script that unconditionally raises a Lua error. Confirms the error path
/// surfaces through the gRPC stream rather than hanging or panicking the
/// server. The current server implementation (core.rs) emits a LogError
/// via the Lua error hook; a future revision may add CompleteError — this
/// test accepts either.
#[tokio::test]
async fn grpc_script_error_propagates() {
    let mut server = TestServer::start("grpc_error").await.expect("server up");

    let tmp = tempfile::tempdir().expect("tempdir");
    let destination = tmp.path().to_string_lossy().to_string();
    let (tx, mut stream) = server.open_stream().await.expect("open stream");
    tx.send(msg::initialize(destination, String::new()))
        .await
        .expect("initialize send");

    // Drain until we see either CompleteError or a LogError that mentions
    // our sentinel string, or we hit the timeout in `next()`.
    let mut saw_error_signal = false;
    for _ in 0..16 {
        match next(&mut stream).await {
            SMessage::CompleteError(_) => {
                saw_error_signal = true;
                break;
            }
            SMessage::LogError(m) if m.contains("intentional test failure") => {
                saw_error_signal = true;
                break;
            }
            SMessage::CompleteSuccess(_) => {
                panic!("expected error propagation, got CompleteSuccess");
            }
            _ => {}
        }
    }
    assert!(
        saw_error_signal,
        "expected CompleteError or LogError(intentional test failure) on the gRPC stream"
    );
}

/// Phase 3 of federated-catalog: a local CatalogIndexer that encounters a
/// `server:` entry should fetch the remote tree via BrowseCatalog and
/// splice the children in with `path` prefixed and `remote` populated.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn remote_indexer_federates_server_entry() {
    // Stand up a real server with a nested catalog that the indexer will fetch.
    let inner = build_catalog(&[("basic", "grpc_basic")]);
    let (services_name, services_entry) = build_nested_entry("services", inner);
    let mut server_catalog = LinkedHashMap::new();
    server_catalog.insert(services_name, services_entry);

    let server = TestServer::start_with_catalog(server_catalog)
        .await
        .expect("server up");
    let endpoint = format!("http://127.0.0.1:{}", server.port);

    // Local config: a single top-level entry called "acme" that federates
    // to the server we just started.
    let mut local_catalog = LinkedHashMap::new();
    local_catalog.insert(
        "acme".to_string(),
        CatalogEntry {
            description: Some("Federated".to_string()),
            source: None,
            catalog: None,
            server: Some(CatalogEntryServer {
                endpoint: endpoint.clone(),
                tls: None,
            }),
            answers: None,
            switches: None,
            use_defaults: None,
            use_defaults_all: None,
            library: false,
            show: true,
        },
    );
    let config = Configuration::default().with_catalog(local_catalog.clone());
    let archetect = Archetect::builder()
        .with_configuration(config)
        .with_temp_layout()
        .expect("temp layout")
        .build()
        .expect("archetect build");

    // Run the indexer on a blocking thread — it creates its own tokio
    // runtime internally for the gRPC fetch.
    let index = tokio::task::spawn_blocking(move || {
        CatalogIndexer::new(archetect).build_index(&local_catalog)
    })
    .await
    .expect("indexer task");

    // Root has one entry: "acme", flagged as remote.
    let roots = index.root();
    assert_eq!(roots.len(), 1);
    let acme = &roots[0];
    assert_eq!(acme.name, "acme");
    assert!(acme.remote.is_some(), "acme should be flagged remote");

    // The children from the server (services → basic) should be spliced
    // under acme with paths rewritten.
    let services = index
        .get("acme/services")
        .expect("services path should resolve under acme");
    assert_eq!(services.name, "services");
    assert!(
        services.remote.is_some(),
        "descendants of a server entry carry remote info"
    );

    let basic = index
        .get("acme/services/basic")
        .expect("basic path should resolve under acme/services");
    assert_eq!(basic.name, "basic");
    // The remote's view of this path is "services/basic" — the bit after
    // the local prefix.
    assert_eq!(basic.remote_path(), Some("services/basic".to_string()));
    assert_eq!(
        basic.remote.as_ref().map(|r| r.endpoint.as_str()),
        Some(endpoint.as_str())
    );
}

/// Phase 1 of federated-catalog: BrowseCatalog returns the catalog tree
/// rooted at a given path. Also exercises SearchCatalog on the same
/// server instance so we know both RPCs wire up correctly.
#[tokio::test]
async fn grpc_browse_and_search_catalog() {
    use archetect_core::proto::grpc::BrowseCatalogRequest;
    use archetect_core::proto::grpc::SearchCatalogRequest;

    let inner = build_catalog(&[("basic", "grpc_basic")]);
    let (name, services) = build_nested_entry("services", inner);
    let mut catalog = LinkedHashMap::new();
    catalog.insert(name, services);

    let mut server = TestServer::start_with_catalog(catalog)
        .await
        .expect("server up");

    // Browse root — expect the "services" group with its child leaf eagerly included.
    let resp = server
        .client
        .browse_catalog(BrowseCatalogRequest {
            path: String::new(),
        })
        .await
        .expect("browse root")
        .into_inner();
    assert_eq!(resp.entries.len(), 1, "expected one root entry");
    let services = &resp.entries[0];
    assert_eq!(services.name, "services");
    assert_eq!(services.path, "services");
    assert_eq!(services.children.len(), 1, "services group has one child");
    let basic = &services.children[0];
    assert_eq!(basic.name, "basic");
    assert_eq!(basic.path, "services/basic");

    // Browse into a path — should return the sub-entries directly.
    let resp = server
        .client
        .browse_catalog(BrowseCatalogRequest {
            path: "services".to_string(),
        })
        .await
        .expect("browse services")
        .into_inner();
    assert_eq!(resp.entries.len(), 1);
    assert_eq!(resp.entries[0].name, "basic");

    // Search for a term that only matches the leaf.
    let resp = server
        .client
        .search_catalog(SearchCatalogRequest {
            query: "basic".to_string(),
            include_hidden: false,
        })
        .await
        .expect("search")
        .into_inner();
    assert!(
        resp.results.iter().any(|e| e.name == "basic"),
        "expected search to return the basic entry, got {:?}",
        resp.results.iter().map(|e| &e.name).collect::<Vec<_>>()
    );
}

/// Phase 0 of the federated-catalog plan: Initialize carries a
/// `catalog_path` field. The server walks the configured catalog tree by
/// path and renders that specific entry instead of the "default" fallback.
/// Tested with a nested group to prove path traversal is recursive.
#[tokio::test]
async fn grpc_catalog_path_selects_nested_entry() {
    // Build a catalog: services/basic → grpc_basic fixture.
    // Also seed a sibling "default" entry pointing at the error fixture so
    // we can prove we're NOT falling back to it when catalog_path is set.
    let basic_fixture = build_catalog(&[("basic", "grpc_basic")]);
    let (name, services) = build_nested_entry("services", basic_fixture);

    let mut catalog = LinkedHashMap::new();
    // `default` points at the error fixture — we'd see an error signal
    // bubble through if the server fell back to it.
    for (k, v) in build_catalog(&[("default", "grpc_error")]) {
        catalog.insert(k, v);
    }
    catalog.insert(name, services);

    let mut server = TestServer::start_with_catalog(catalog)
        .await
        .expect("server up");

    let tmp = tempfile::tempdir().expect("tempdir");
    let destination = tmp.path().to_string_lossy().to_string();
    let (tx, mut stream) = server.open_stream().await.expect("open stream");
    tx.send(msg::initialize_with_path(
        destination,
        String::new(),
        "services/basic".to_string(),
    ))
    .await
    .expect("initialize send");

    // grpc_basic's first ScriptMessage is a PromptForText("Name:"). If the
    // server had fallen back to "default" (the error fixture), we'd see
    // either a LogError or CompleteError first.
    loop {
        match next(&mut stream).await {
            SMessage::PromptForText(p) => {
                assert_eq!(p.message, "Name:");
                tx.send(msg::string("path-routed".to_string()))
                    .await
                    .expect("send string");
                break;
            }
            SMessage::CompleteError(err) => {
                panic!(
                    "catalog_path resolution fell back to default/error fixture: {:?}",
                    err
                );
            }
            SMessage::LogError(m) if m.contains("intentional test failure") => {
                panic!(
                    "catalog_path resolution fell back to default/error fixture: {}",
                    m
                );
            }
            // Display/LogInfo/LogDebug — skip, not determinative.
            _ => {}
        }
    }

    // Drain the rest of the stream until completion so we exercise the
    // full render path, not just the prompt.
    loop {
        match next(&mut stream).await {
            SMessage::WriteDirectory(_) | SMessage::WriteFile(_) => {
                tx.send(msg::ack()).await.expect("ack");
            }
            SMessage::CompleteSuccess(_) => break,
            SMessage::CompleteError(err) => panic!("render failed: {:?}", err),
            _ => {}
        }
    }
}

/// Client retry: pointing at a dead port should fail fast under the
/// configured retry budget rather than panicking or hanging. This is the
/// core contract of Phase 5 client reconnection.
#[tokio::test]
async fn grpc_client_retry_exhausts_and_errors_cleanly() {
    // Bind and immediately drop a listener to grab a free port that's
    // guaranteed to refuse connections for the rest of the test.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind to ephemeral port");
    let port = listener.local_addr().expect("local addr").port();
    drop(listener);

    let endpoint = format!("http://127.0.0.1:{}", port);

    // Keep options tight so the test finishes quickly. 2 retries × 10ms base
    // backoff yields <100ms total — well under the test default timeout.
    let options = ClientOptions {
        connect_timeout: Duration::from_millis(100),
        max_connect_retries: 2,
        connect_backoff_base: Duration::from_millis(10),
        max_backoff: Duration::from_millis(50),
        ..ClientOptions::default()
    };

    let render_context = archetect_core::archetype::render_context::RenderContext::new(
        camino::Utf8PathBuf::from("/tmp/archetect-client-retry-test"),
        archetect_api::ContextMap::new(),
    );

    // start_with_options is sync (spins up its own runtime). Move it off
    // this test's runtime thread so we don't nest runtimes.
    let result = tokio::task::spawn_blocking(move || {
        archetect_core::client::start_with_options(render_context, endpoint, options)
    })
    .await
    .expect("spawn_blocking");

    let err = result.expect_err("expected connection failure");
    let msg = err.to_string();
    assert!(
        msg.contains("failed to connect"),
        "unexpected error message: {}",
        msg
    );
}

/// Phase 5 graceful shutdown: `server.shutdown()` should cause a running
/// `serve()` call to return Ok. Previously `ArchetectServer::serve()` had
/// no shutdown path and would only stop when the process exited.
#[tokio::test]
async fn grpc_server_graceful_shutdown() {
    use archetect_core::configuration::Configuration;
    use archetect_core::server::{ArchetectServer, ArchetectServiceCore};
    use archetect_core::Archetect;

    let configuration = Configuration::default();
    let prototype = Archetect::builder()
        .with_configuration(configuration)
        .with_temp_layout()
        .expect("temp layout")
        .build()
        .expect("archetect build");

    let core = ArchetectServiceCore::builder(prototype)
        .build()
        .await
        .expect("core build");

    let server = ArchetectServer::builder(core)
        .with_host("127.0.0.1".to_string())
        .with_port(0)
        .build()
        .await
        .expect("server build");

    let server_handle = server.clone();
    let serve_task = tokio::spawn(async move { server_handle.serve().await });

    // Give serve() a moment to actually start listening.
    tokio::time::sleep(Duration::from_millis(50)).await;

    server.shutdown().await;

    let result = tokio::time::timeout(Duration::from_secs(5), serve_task)
        .await
        .expect("serve() did not return within 5s of shutdown")
        .expect("serve task panicked");
    result.expect("serve() returned error on clean shutdown");
}
