use std::path::PathBuf;
use std::time::Duration;

use camino::Utf8PathBuf;
use linked_hash_map::LinkedHashMap;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::transport::Channel;

use archetect_core::configuration::Configuration;
use archetect_core::manifest::CatalogEntry;
use archetect_core::proto::grpc;
use archetect_core::proto::grpc::archetect_service_client::ArchetectServiceClient;
use archetect_core::server::{ArchetectServer, ArchetectServiceCore};
use archetect_core::Archetect;

/// Returns the absolute filesystem path to the named fixture directory under
/// `tests/grpc/fixtures/`. Used to seed the server's catalog "default"
/// entry so the render path can pick it up.
pub fn fixture_path(name: &str) -> Utf8PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests");
    path.push("grpc");
    path.push("fixtures");
    path.push(name);
    Utf8PathBuf::from_path_buf(path).expect("utf8 path")
}

/// Build a flat catalog from `(entry_name, fixture_name)` pairs.
pub fn build_catalog(entries: &[(&str, &str)]) -> LinkedHashMap<String, CatalogEntry> {
    let mut catalog = LinkedHashMap::new();
    for (name, fixture) in entries {
        catalog.insert(
            (*name).to_string(),
            CatalogEntry {
                description: Some(format!("Test fixture: {}", fixture)),
                source: Some(fixture_path(fixture).to_string()),
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
    }
    catalog
}

/// Build a nested sub-catalog as a single entry (name + inner children).
/// Used to exercise multi-segment catalog paths.
pub fn build_nested_entry(
    name: &str,
    children: LinkedHashMap<String, CatalogEntry>,
) -> (String, CatalogEntry) {
    (
        name.to_string(),
        CatalogEntry {
            description: Some(format!("Group: {}", name)),
            source: None,
            catalog: Some(children),
            answers: None,
            switches: None,
            use_defaults: None,
            use_defaults_all: None,
            server: None,
            library: false,
            show: true,
        },
    )
}

/// A running test server + connected client pair. The server's
/// `serve_with_incoming` task is owned by the harness; dropping `TestServer`
/// aborts it and the client connection goes with it.
pub struct TestServer {
    #[allow(dead_code)] // exposed for future tests that assert on the bound port
    pub port: u16,
    pub client: ArchetectServiceClient<Channel>,
    _server_task: tokio::task::JoinHandle<()>,
}

impl TestServer {
    /// Boot an `ArchetectServer` with a configuration whose catalog has a
    /// single entry named "default" pointing at the given fixture. The server
    /// picks that entry up in its current bootstrap flow (see
    /// `archetect-core/src/server/core.rs`).
    pub async fn start(fixture: &str) -> anyhow::Result<Self> {
        Self::start_with_catalog(build_catalog(&[("default", fixture)])).await
    }

    /// Like `start`, but lets the test declare an arbitrary catalog layout —
    /// useful for path-based resolution tests that need named or nested entries.
    pub async fn start_with_catalog(
        catalog: LinkedHashMap<String, CatalogEntry>,
    ) -> anyhow::Result<Self> {

        // A minimal Archetect for the server prototype. Temp layout keeps
        // the cache/state out of the user's XDG dirs.
        let configuration = Configuration::default().with_catalog(catalog);
        let prototype = Archetect::builder()
            .with_configuration(configuration)
            .with_temp_layout()?
            .build()?;

        let core = ArchetectServiceCore::builder(prototype).build().await?;

        let server = ArchetectServer::builder(core)
            .with_host("127.0.0.1".to_string())
            .with_port(0)
            .build()
            .await?;
        let port = server.service_port();

        let server_task = tokio::spawn(async move {
            // Errors here surface when the serve task is dropped (the test
            // ended) — we don't want that to fail the test.
            let _ = server.serve().await;
        });

        // Poll-connect: the `serve()` task runs on a separate tokio task and
        // may not be listening when `connect()` is first attempted.
        let endpoint = format!("http://127.0.0.1:{}", port);
        let client = {
            let mut attempts = 0;
            loop {
                match ArchetectServiceClient::connect(endpoint.clone()).await {
                    Ok(c) => break c,
                    Err(e) => {
                        attempts += 1;
                        if attempts >= 20 {
                            return Err(anyhow::anyhow!(
                                "failed to connect after {} attempts: {:?}",
                                attempts,
                                e
                            ));
                        }
                        tokio::time::sleep(Duration::from_millis(50)).await;
                    }
                }
            }
        };

        Ok(TestServer {
            port,
            client,
            _server_task: server_task,
        })
    }

    /// Open a bidirectional streaming call. Returns the server->client
    /// receive stream plus a sender that the test drives to talk back.
    pub async fn open_stream(
        &mut self,
    ) -> anyhow::Result<(
        mpsc::Sender<grpc::ClientMessage>,
        tonic::Streaming<grpc::ScriptMessage>,
    )> {
        let (client_tx, client_rx) = mpsc::channel(32);
        let stream = ReceiverStream::new(client_rx);
        let response = self.client.streaming_api(stream).await?;
        Ok((client_tx, response.into_inner()))
    }
}

/// Helpers for building the grpc message variants the tests need. Mirrors
/// `archetect_api::ClientMessage` but produces the proto type directly so
/// the tests aren't dependent on the conversion layer they're validating.
pub mod msg {
    use super::grpc;

    pub fn initialize(destination: String, answers_yaml: String) -> grpc::ClientMessage {
        initialize_with_path(destination, answers_yaml, String::new())
    }

    pub fn initialize_with_path(
        destination: String,
        answers_yaml: String,
        catalog_path: String,
    ) -> grpc::ClientMessage {
        grpc::ClientMessage {
            message: Some(grpc::client_message::Message::Initialize(
                grpc::Initialize {
                    answers_yaml,
                    switches: Vec::new(),
                    use_defaults: Vec::new(),
                    use_defaults_all: false,
                    destination,
                    catalog_path,
                },
            )),
        }
    }

    pub fn string(s: String) -> grpc::ClientMessage {
        grpc::ClientMessage {
            message: Some(grpc::client_message::Message::String(s)),
        }
    }

    pub fn ack() -> grpc::ClientMessage {
        grpc::ClientMessage {
            message: Some(grpc::client_message::Message::Ack(())),
        }
    }
}
