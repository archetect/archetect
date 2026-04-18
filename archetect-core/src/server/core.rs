use std::pin::Pin;
use std::time::Duration;

use archetect_api::ContextMap;
use linked_hash_map::LinkedHashMap;
use tokio::sync::mpsc;
use tokio::time::sleep;
use tokio_stream::{Stream, StreamExt};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status, Streaming};
use tracing::{error, info, warn, Instrument};
use uuid::Uuid;

use crate::catalog::catalog_index::{IndexEntry, IndexEntryKind};
use crate::catalog::catalog_indexer::CatalogIndexer;
use crate::manifest::CatalogEntry;

use archetect_api::ScriptMessage;

use crate::archetype::render_context::RenderContext;
use crate::errors::ArchetectError;
use crate::io::AsyncScriptIoHandle;
use crate::proto::grpc;
use crate::proto::grpc::archetect_service_server::ArchetectService;
use crate::Archetect;

#[derive(Clone, Debug)]
pub struct ArchetectServiceCore {
    prototype: Archetect,
}

impl ArchetectServiceCore {
    pub fn builder(prototype: Archetect) -> ArchetectServiceCoreBuilder {
        ArchetectServiceCoreBuilder::new(prototype)
    }

    pub fn prototype(&self) -> &Archetect {
        &self.prototype
    }
}

pub struct ArchetectServiceCoreBuilder {
    prototype: Archetect,
}

impl ArchetectServiceCoreBuilder {
    pub fn new(prototype: Archetect) -> Self {
        Self { prototype }
    }

    pub async fn build(self) -> Result<ArchetectServiceCore, ArchetectError> {
        Ok(ArchetectServiceCore {
            prototype: self.prototype,
        })
    }
}

type ResponseStream =
    Pin<Box<dyn Stream<Item = Result<grpc::ScriptMessage, Status>> + Send>>;

#[tonic::async_trait]
impl ArchetectService for ArchetectServiceCore {
    type StreamingApiStream = ResponseStream;

    async fn streaming_api(
        &self,
        request: Request<Streaming<grpc::ClientMessage>>,
    ) -> Result<Response<Self::StreamingApiStream>, Status> {
        // Request-scoped ID so every log line this stream emits can be
        // correlated end-to-end (connect → render → complete/error).
        let request_id = Uuid::new_v4();
        let peer = request
            .remote_addr()
            .map(|a| a.to_string())
            .unwrap_or_else(|| "<unknown>".to_string());
        let stream_span = tracing::info_span!(
            "grpc_stream",
            request_id = %request_id,
            peer = %peer,
        );
        let _enter = stream_span.clone().entered();
        info!("Archetect Bidirectional Streaming API Initiating");

        let mut in_stream = request.into_inner();

        let (client_tx, client_rx) = mpsc::channel(10);
        let (script_tx, script_rx) = mpsc::channel(10);
        let client_failure_tx = client_tx.clone();

        let script_handle = AsyncScriptIoHandle::from_channels(script_tx, client_rx);
        let archetect = Archetect::builder()
            .with_configuration(self.prototype().configuration().clone())
            .with_driver(script_handle)
            .build()
            .map_err(|e| Status::internal(format!("Failed to initialize Archetect: {}", e)))?;

        let mut archetect_handle = None;
        let mut initialized = false;

        let task_span = stream_span.clone();
        tokio::spawn(async move {
            while let Some(message) = in_stream.next().await {
                match message {
                    Ok(message) => {
                        if !initialized {
                            let archetect = archetect.clone();
                            archetect_handle = Some(tokio::task::spawn_blocking(move || {
                                if let grpc::ClientMessage {
                                    message:
                                        Some(grpc::client_message::Message::Initialize(initialize)),
                                } = message
                                {
                                    let answers = serde_yaml::from_str::<ContextMap>(
                                        &initialize.answers_yaml,
                                    )
                                    .unwrap_or_else(|err| {
                                        warn!("Failed to parse answers YAML: {}", err);
                                        ContextMap::new()
                                    });

                                    let destination = initialize.destination;
                                    // Resolve the source. Priority order:
                                    //   1. Initialize.catalog_path — follow the slash-
                                    //      separated path into the server's catalog
                                    //      tree (federation case).
                                    //   2. Catalog entry named "default".
                                    //   3. First entry in the catalog with a source
                                    //      (legacy fallback).
                                    let source = archetect
                                        .configuration()
                                        .catalog()
                                        .and_then(|catalog| {
                                            if !initialize.catalog_path.is_empty() {
                                                resolve_source_by_path(catalog, &initialize.catalog_path)
                                            } else {
                                                catalog
                                                    .get("default")
                                                    .and_then(|e| e.source.clone())
                                                    .or_else(|| {
                                                        catalog
                                                            .values()
                                                            .find_map(|e| e.source.clone())
                                                    })
                                            }
                                        });

                                    if let Some(source) = source {
                                        let render_context = RenderContext::new(destination, answers)
                                            .with_switches(
                                                initialize
                                                    .switches
                                                    .iter()
                                                    .map(|v| v.to_string())
                                                    .collect(),
                                            )
                                            .with_use_defaults(
                                                initialize
                                                    .use_defaults
                                                    .iter()
                                                    .map(|v| v.to_string())
                                                    .collect(),
                                            )
                                            .with_use_defaults_all(initialize.use_defaults_all);

                                        match archetect.new_archetype(&source) {
                                            Ok(archetype) => {
                                                match archetype.render(render_context) {
                                                    Ok(_) => {
                                                        info!("Successfully rendered");
                                                        let _ = archetect.request(
                                                            ScriptMessage::CompleteSuccess,
                                                        );
                                                    }
                                                    Err(err) => {
                                                        error!("Render error: {:?}", err);
                                                    }
                                                }
                                            }
                                            Err(err) => {
                                                let _ = archetect.request(
                                                    ScriptMessage::CompleteError(err.to_string()),
                                                );
                                            }
                                        }
                                    } else {
                                        let _ = archetect.request(ScriptMessage::CompleteError(
                                            "No default action configured".to_string(),
                                        ));
                                    }
                                } else {
                                    let _ = archetect.request(ScriptMessage::LogError(
                                        "Improper Initialization Message".to_string(),
                                    ));
                                }
                            }));

                            initialized = true;
                        } else {
                            let _ = client_tx.send(message).await;
                        }
                    }
                    Err(err) => {
                        warn!("gRPC Error: {}. Sending Abort Message", err);
                        let _ = client_failure_tx
                            .send(grpc::ClientMessage {
                                message: Some(grpc::client_message::Message::Abort(())),
                            })
                            .await;
                    }
                }
            }

            if let Some(handle) = archetect_handle {
                tokio::select! {
                    _ = handle => {
                        info!("Archetect thread closed successfully");
                    },
                    _ = sleep(Duration::from_secs(30)) => {
                        error!("Archetect thread failed to close within 30 seconds");
                    }
                };
            }
            info!("Client disconnected");
        }.instrument(task_span));

        let out_stream = ReceiverStream::new(script_rx).map(Ok);

        Ok(Response::new(
            Box::pin(out_stream) as Self::StreamingApiStream
        ))
    }

    async fn browse_catalog(
        &self,
        request: Request<grpc::BrowseCatalogRequest>,
    ) -> Result<Response<grpc::BrowseCatalogResponse>, Status> {
        let req = request.into_inner();
        let path = req.path;
        let archetect = self.prototype.clone();

        // Building the catalog index resolves nested sources, which can be
        // I/O bound (git pulls, filesystem walks). Do it on a blocking pool
        // so we don't stall the tokio reactor.
        let entries = tokio::task::spawn_blocking(move || {
            let Some(catalog) = archetect.configuration().catalog().cloned() else {
                return Vec::new();
            };
            let index = CatalogIndexer::new(archetect).build_index(&catalog);
            match index.browse(&path) {
                Some(slice) => slice.iter().map(index_entry_to_proto).collect(),
                None => Vec::new(),
            }
        })
        .await
        .map_err(|err| Status::internal(format!("browse_catalog task failed: {}", err)))?;

        Ok(Response::new(grpc::BrowseCatalogResponse { entries }))
    }

    async fn search_catalog(
        &self,
        request: Request<grpc::SearchCatalogRequest>,
    ) -> Result<Response<grpc::SearchCatalogResponse>, Status> {
        let req = request.into_inner();
        let query = req.query;
        let include_hidden = req.include_hidden;
        let archetect = self.prototype.clone();

        let results = tokio::task::spawn_blocking(move || {
            let Some(catalog) = archetect.configuration().catalog().cloned() else {
                return Vec::new();
            };
            let index = CatalogIndexer::new(archetect).build_index(&catalog);
            index
                .search(&query)
                .into_iter()
                .filter(|e| include_hidden || e.show)
                .map(index_entry_to_proto)
                .collect()
        })
        .await
        .map_err(|err| Status::internal(format!("search_catalog task failed: {}", err)))?;

        Ok(Response::new(grpc::SearchCatalogResponse { results }))
    }
}

/// Convert an `IndexEntry` (with its full subtree) into the proto wire
/// format. Children are included verbatim so clients get a browsable tree
/// from one RPC.
fn index_entry_to_proto(entry: &IndexEntry) -> grpc::CatalogIndexEntry {
    let kind = match entry.kind {
        IndexEntryKind::Group => grpc::CatalogEntryKind::Group,
        IndexEntryKind::Leaf => grpc::CatalogEntryKind::Leaf,
    };
    grpc::CatalogIndexEntry {
        path: entry.path.clone(),
        name: entry.name.clone(),
        description: entry.description.clone(),
        kind: kind as i32,
        is_archetype: entry.is_archetype,
        has_source: entry.source.is_some(),
        show: entry.show,
        children: entry.children.iter().map(index_entry_to_proto).collect(),
    }
}

/// Walk a slash-separated catalog path to its leaf entry and return that
/// entry's source. Nested sub-catalogs are traversed via `CatalogEntry::catalog`.
/// Returns None if any segment is missing or if the leaf has no source.
fn resolve_source_by_path(
    catalog: &LinkedHashMap<String, CatalogEntry>,
    path: &str,
) -> Option<String> {
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    if segments.is_empty() {
        return None;
    }
    let mut current: &LinkedHashMap<String, CatalogEntry> = catalog;
    let mut entry: Option<&CatalogEntry> = None;
    for (i, segment) in segments.iter().enumerate() {
        let found = current.get(*segment)?;
        if i == segments.len() - 1 {
            entry = Some(found);
        } else {
            current = found.catalog.as_ref()?;
        }
    }
    entry.and_then(|e| e.source.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::CatalogEntry;

    fn leaf(name: &str, source: &str) -> (String, CatalogEntry) {
        (
            name.to_string(),
            CatalogEntry {
                description: Some(name.to_string()),
                source: Some(source.to_string()),
                catalog: None,
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

    fn group(name: &str, children: LinkedHashMap<String, CatalogEntry>) -> (String, CatalogEntry) {
        (
            name.to_string(),
            CatalogEntry {
                description: Some(name.to_string()),
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

    #[test]
    fn resolves_nested_path() {
        let mut services = LinkedHashMap::new();
        let (n, e) = leaf("grpc", "git://example.com/grpc.git");
        services.insert(n, e);
        let mut root = LinkedHashMap::new();
        let (n, e) = group("services", services);
        root.insert(n, e);

        assert_eq!(
            resolve_source_by_path(&root, "services/grpc"),
            Some("git://example.com/grpc.git".to_string())
        );
    }

    #[test]
    fn resolves_top_level_path() {
        let mut root = LinkedHashMap::new();
        let (n, e) = leaf("default", "git://example.com/default.git");
        root.insert(n, e);
        assert_eq!(
            resolve_source_by_path(&root, "default"),
            Some("git://example.com/default.git".to_string())
        );
    }

    #[test]
    fn returns_none_for_missing_segment() {
        let mut root = LinkedHashMap::new();
        let (n, e) = leaf("default", "git://example.com/default.git");
        root.insert(n, e);
        assert!(resolve_source_by_path(&root, "missing").is_none());
        assert!(resolve_source_by_path(&root, "default/extra").is_none());
    }

    #[test]
    fn returns_none_for_empty_path() {
        let mut root = LinkedHashMap::new();
        let (n, e) = leaf("default", "git://example.com/default.git");
        root.insert(n, e);
        assert!(resolve_source_by_path(&root, "").is_none());
    }
}
