//! Verifies the TLS plumbing end-to-end: server configured with a
//! self-signed cert, client configured to trust that cert, and a
//! subsequent gRPC call succeeds. Also verifies the negative case —
//! a plaintext client rejects the TLS server's handshake.

use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use linked_hash_map::LinkedHashMap;
use rcgen::{CertifiedKey, generate_simple_self_signed};
use tokio_stream::StreamExt;

use archetect_core::client::{ClientOptions, ClientTlsOptions};
use archetect_core::configuration::Configuration;
use archetect_core::manifest::CatalogEntry;
use archetect_core::proto::grpc::archetect_service_client::ArchetectServiceClient;
use archetect_core::server::{ArchetectServer, ArchetectServiceCore, TlsConfig};
use archetect_core::Archetect;

use super::harness::fixture_path;

/// Helper: generate a self-signed cert + key pair for the given SAN. Returns
/// (cert_pem_path, key_pem_path) — both written under `dir`.
fn write_self_signed(dir: &std::path::Path, san: &str) -> anyhow::Result<(PathBuf, PathBuf)> {
    let CertifiedKey { cert, signing_key } = generate_simple_self_signed([san.to_string()])?;
    let cert_path = dir.join("server.crt");
    let key_path = dir.join("server.key");
    fs::write(&cert_path, cert.pem())?;
    fs::write(&key_path, signing_key.serialize_pem())?;
    Ok((cert_path, key_path))
}

/// Build a minimal Archetect prototype seeded with a "default" catalog
/// entry pointing at the given fixture.
fn prototype_with_fixture(fixture: &str) -> anyhow::Result<Archetect> {
    let archetype_path = fixture_path(fixture);
    let mut catalog = LinkedHashMap::new();
    catalog.insert(
        "default".to_string(),
        CatalogEntry {
            description: Some(format!("TLS fixture: {}", fixture)),
            source: Some(archetype_path.to_string()),
            catalog: None,
            answers: None,
            switches: None,
            use_defaults: None,
            use_defaults_all: None,
            server: None,
            library: false,
            show: true,
        },
    );
    let configuration = Configuration::default().with_catalog(catalog);
    Ok(Archetect::builder()
        .with_configuration(configuration)
        .with_temp_layout()?
        .build()?)
}

/// Verifies that the TLS pipe — self-signed cert on the server side,
/// trust-store override on the client side — completes a handshake and
/// opens a bidirectional streaming RPC. We deliberately stop short of
/// driving a full render; the non-TLS tests cover the render protocol,
/// and the only thing this test adds is "can the transport carry it at
/// all when wrapped in TLS?"
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn grpc_tls_handshake_and_stream_open() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let (cert_path, key_path) =
        write_self_signed(tmp.path(), "localhost").expect("generate self-signed cert");

    let prototype = prototype_with_fixture("grpc_basic").expect("prototype");
    let core = ArchetectServiceCore::builder(prototype)
        .build()
        .await
        .expect("core");

    let server = ArchetectServer::builder(core)
        .with_host("127.0.0.1".to_string())
        .with_port(0)
        .with_tls(TlsConfig {
            cert_path: cert_path.clone(),
            key_path: key_path.clone(),
            client_ca_path: None,
        })
        .build()
        .await
        .expect("server with TLS");
    let port = server.service_port();

    let server_task = tokio::spawn({
        let server = server.clone();
        async move {
            let _ = server.serve().await;
        }
    });
    tokio::time::sleep(Duration::from_millis(100)).await;

    let endpoint_url = format!("https://127.0.0.1:{}", port);
    let tls_opts = ClientTlsOptions {
        ca_cert_path: Some(cert_path.clone()),
        client_cert_path: None,
        client_key_path: None,
        // The cert's SAN is "localhost" but we connect via 127.0.0.1.
        domain_name: Some("localhost".to_string()),
    };

    let mut ep = tonic::transport::Endpoint::from_shared(endpoint_url)
        .expect("endpoint parse")
        .connect_timeout(Duration::from_secs(2));
    ep = ep
        .tls_config(build_test_client_tls(&tls_opts).expect("tls"))
        .expect("apply tls config");

    let channel = tokio::time::timeout(Duration::from_secs(5), ep.connect())
        .await
        .expect("TLS connect did not complete in time")
        .expect("TLS handshake failed against self-signed server");
    let mut client = ArchetectServiceClient::new(channel);

    // Open the streaming RPC. Send nothing — a server that actually went
    // through the TLS handshake will hold the stream open awaiting an
    // Initialize frame. A server that rejected the TLS handshake would
    // have already errored above.
    let (_tx, client_rx) = tokio::sync::mpsc::channel::<
        archetect_core::proto::grpc::ClientMessage,
    >(1);
    let stream = tokio_stream::wrappers::ReceiverStream::new(client_rx);
    let mut response_stream = tokio::time::timeout(
        Duration::from_secs(5),
        client.streaming_api(stream),
    )
    .await
    .expect("streaming_api() did not return within 5s")
    .expect("streaming_api returned an error")
    .into_inner();

    // Pull once with a short timeout. We expect nothing (no Initialize sent),
    // so a clean timeout is the signal the stream is healthy. If the server
    // had a TLS-related problem we'd instead see an RST or tonic Status
    // error here.
    match tokio::time::timeout(Duration::from_millis(300), response_stream.next()).await {
        Err(_) => { /* expected — stream is open, no messages yet */ }
        Ok(Some(Ok(sm))) => {
            // Server surfaced something unexpected (e.g. a LogError); that's
            // informational, not a TLS failure. Print for diagnostics but
            // accept it as proof the stream is alive.
            eprintln!(
                "TLS stream open; received unsolicited ScriptMessage: {:?}",
                sm.message
            );
        }
        Ok(Some(Err(status))) => panic!("stream errored post-handshake: {:?}", status),
        Ok(None) => panic!("stream closed immediately after TLS handshake"),
    }

    // Drop the stream + channel explicitly so the server's in-flight task
    // sees its receiver close before we signal shutdown.
    drop(response_stream);
    drop(client);

    server.shutdown().await;
    // Don't await the server task — we've already verified shutdown()
    // delivered the signal, and the task owns a TLS session that tonic
    // won't unwind cleanly with a half-open peer.
    server_task.abort();
}

#[tokio::test]
async fn grpc_plaintext_client_rejected_by_tls_server() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let (cert_path, key_path) =
        write_self_signed(tmp.path(), "localhost").expect("generate self-signed cert");

    let prototype = prototype_with_fixture("grpc_basic").expect("prototype");
    let core = ArchetectServiceCore::builder(prototype)
        .build()
        .await
        .expect("core");

    let server = ArchetectServer::builder(core)
        .with_host("127.0.0.1".to_string())
        .with_port(0)
        .with_tls(TlsConfig {
            cert_path,
            key_path,
            client_ca_path: None,
        })
        .build()
        .await
        .expect("server build");
    let port = server.service_port();

    let server_task = tokio::spawn({
        let server = server.clone();
        async move {
            let _ = server.serve().await;
        }
    });
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Plain HTTP against TLS-enabled server should fail. Don't care whether
    // the failure surfaces at connect time or first RPC — either is fine,
    // as long as it's not a silent success.
    let endpoint_url = format!("http://127.0.0.1:{}", port);
    let options = ClientOptions {
        connect_timeout: Duration::from_millis(500),
        max_connect_retries: 0,
        connect_backoff_base: Duration::from_millis(10),
        max_backoff: Duration::from_millis(50),
        ..ClientOptions::default()
    };

    let render_context = archetect_core::archetype::render_context::RenderContext::new(
        camino::Utf8PathBuf::from("/tmp/archetect-plaintext-rejected"),
        archetect_api::ContextMap::new(),
    );

    let result = tokio::task::spawn_blocking(move || {
        archetect_core::client::start_with_options(render_context, endpoint_url, options)
    })
    .await
    .expect("spawn_blocking");

    assert!(
        result.is_err(),
        "plaintext client should not succeed against a TLS server"
    );

    server.shutdown().await;
    let _ = server_task.await;
}

/// The same TLS-config builder the production client uses — duplicated here
/// instead of re-exporting from the library (it's a crate-private detail)
/// so this test exercises the same behavior without opening a public seam.
fn build_test_client_tls(
    tls: &ClientTlsOptions,
) -> anyhow::Result<tonic::transport::ClientTlsConfig> {
    use tonic::transport::{Certificate, ClientTlsConfig, Identity};

    let mut config = ClientTlsConfig::new().with_enabled_roots();

    if let Some(ca_path) = &tls.ca_cert_path {
        let ca_pem = fs::read(ca_path)?;
        config = config.ca_certificate(Certificate::from_pem(ca_pem));
    }
    if let (Some(cert_path), Some(key_path)) = (&tls.client_cert_path, &tls.client_key_path) {
        let cert_pem = fs::read(cert_path)?;
        let key_pem = fs::read(key_path)?;
        config = config.identity(Identity::from_pem(cert_pem, key_pem));
    }
    if let Some(domain) = &tls.domain_name {
        config = config.domain_name(domain.clone());
    }
    Ok(config)
}
