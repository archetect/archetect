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
use crate::catalog::dispatch;
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
    /// The catalog path renders target when `Initialize.catalog_path` is empty —
    /// `archetect server <action>`, validated at startup. `None` = no explicit
    /// action: fall back to the catalog's own unambiguous default (see
    /// [`resolve_default_entry`]).
    default_action: Option<String>,
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
    default_action: Option<String>,
}

impl ArchetectServiceCoreBuilder {
    pub fn new(prototype: Archetect) -> Self {
        Self {
            prototype,
            default_action: None,
        }
    }

    pub fn with_default_action(mut self, action: Option<String>) -> Self {
        self.default_action = action;
        self
    }

    pub async fn build(self) -> Result<ArchetectServiceCore, ArchetectError> {
        Ok(ArchetectServiceCore {
            prototype: self.prototype,
            default_action: self.default_action,
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
        let default_action = self.default_action.clone();

        let task_span = stream_span.clone();
        tokio::spawn(async move {
            while let Some(message) = in_stream.next().await {
                match message {
                    Ok(message) => {
                        if !initialized {
                            let archetect = archetect.clone();
                            let default_action = default_action.clone();
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

                                    // Default-deny for anything reaching outside
                                    // the destination: the client enumerates
                                    // what it will allow, and the render is
                                    // refused up front if the archetype needs
                                    // more than that.
                                    archetect.restrict_capabilities(
                                        initialize.capabilities.clone(),
                                    );

                                    let destination = initialize.destination;
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

                                    // Resolve the render target. Priority:
                                    //   1. Initialize.catalog_path — resolved with the
                                    //      SAME walker DescribeArchetype uses, so any
                                    //      path a client can describe, it can render.
                                    //   2. The server's startup action (`archetect
                                    //      server <action>`), validated at startup.
                                    //   3. The catalog's own unambiguous default: a
                                    //      "default" entry, or a single-leaf catalog.
                                    // A caller is NEVER handed an arbitrary entry: an
                                    // unresolvable or ambiguous target is an error
                                    // naming the choices.
                                    match resolve_render_target(
                                        &archetect,
                                        &initialize.catalog_path,
                                        default_action.as_deref(),
                                    ) {
                                        Ok((entry, label)) => {
                                            match dispatch::render_leaf(
                                                &archetect,
                                                &entry,
                                                &label,
                                                render_context,
                                            ) {
                                                Ok(_) => {
                                                    info!("Successfully rendered '{}'", label);
                                                    let artifacts = archetect.artifacts();
                                                    let _ = archetect.request(
                                                        ScriptMessage::CompleteSuccess(
                                                            artifacts,
                                                        ),
                                                    );
                                                }
                                                Err(err) => {
                                                    // The client cannot see this log — it is
                                                    // on the other end of a wire. Without a
                                                    // CompleteError the stream just ends and
                                                    // a failed render is indistinguishable
                                                    // from a successful one.
                                                    error!("Render error: {:?}", err);
                                                    let _ = archetect.request(
                                                        ScriptMessage::CompleteError(
                                                            err.to_string(),
                                                        ),
                                                    );
                                                }
                                            }
                                        }
                                        Err(message) => {
                                            error!("Target resolution error: {}", message);
                                            let _ = archetect.request(
                                                ScriptMessage::CompleteError(message),
                                            );
                                        }
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

    async fn describe_archetype(
        &self,
        request: Request<grpc::DescribeArchetypeRequest>,
    ) -> Result<Response<grpc::DescribeArchetypeResponse>, Status> {
        let req = request.into_inner();
        let path = req.path;
        let explore = req.explore;
        let answers_yaml = req.answers_yaml;
        let switches: std::collections::HashSet<String> = req.switches.into_iter().collect();
        let archetect = self.prototype.clone();

        // Probing executes the archetype's script (against the recording
        // driver — no writes, no exec). Blocking pool, like browse.
        let interface_json = tokio::task::spawn_blocking(move || -> Result<String, String> {
            let catalog = archetect
                .configuration()
                .catalog()
                .cloned()
                .ok_or_else(|| "server has no catalog".to_string())?;
            let source = match crate::catalog::dispatch::walk_path(&archetect, &catalog, &path) {
                Some(crate::catalog::dispatch::PathTarget::Leaf(entry)) => entry
                    .source
                    .ok_or_else(|| format!("catalog entry '{}' has no source", path))?,
                _ => return Err(format!("'{}' is not a catalog leaf on this server", path)),
            };
            // A malformed answer document is the caller's error, not a reason
            // to hand back an interface derived from nothing — say so.
            let answers: archetect_api::ContextMap = if answers_yaml.trim().is_empty() {
                archetect_api::ContextMap::new()
            } else {
                serde_yaml::from_str(&answers_yaml)
                    .map_err(|e| format!("answers_yaml is not a YAML mapping: {}", e))?
            };
            let options = crate::interface::ProbeOptions {
                explore,
                answers,
                switches,
                ..Default::default()
            };
            let layout_factory = || -> Result<Box<dyn crate::system::SystemLayout>, crate::errors::ArchetectError> {
                Ok(Box::new(crate::system::XdgSystemLayout::new()?))
            };
            let derived = crate::interface::probe_interface(&archetect, &layout_factory, &source, &options)
                .map_err(|e| format!("probe failed: {}", e))?;
            serde_json::to_string(&derived).map_err(|e| format!("serialize: {}", e))
        })
        .await
        .map_err(|err| Status::internal(format!("describe_archetype task failed: {}", err)))?
        .map_err(Status::failed_precondition)?;

        Ok(Response::new(grpc::DescribeArchetypeResponse { interface_json }))
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

/// Resolve what a streaming render targets: the client's `catalog_path` if it
/// sent one, else the server's startup action, else the catalog's own
/// unambiguous default. Returns the entry plus the path label it resolved
/// from (for error messages and entry-flag reporting).
///
/// Paths resolve through [`dispatch::walk_path`] — the same walker
/// `DescribeArchetype` uses — so describe and render cannot disagree about
/// what a path names. Anything other than a renderable leaf is an error
/// naming the problem; a caller is never silently handed a different entry.
fn resolve_render_target(
    archetect: &Archetect,
    catalog_path: &str,
    default_action: Option<&str>,
) -> Result<(CatalogEntry, String), String> {
    let catalog = archetect
        .configuration()
        .catalog()
        .ok_or_else(|| "This server has no catalog configured".to_string())?;

    let requested = if !catalog_path.is_empty() {
        Some(catalog_path)
    } else {
        default_action
    };

    match requested {
        Some(path) => match dispatch::walk_path(archetect, catalog, path) {
            Some(dispatch::PathTarget::Leaf(entry)) => Ok((entry, path.to_string())),
            Some(dispatch::PathTarget::Group(_)) => Err(format!(
                "Catalog path '{}' is a group on this server — name a renderable leaf",
                path
            )),
            Some(dispatch::PathTarget::Remote { .. }) => Err(format!(
                "Catalog path '{}' is a federated entry on this server — dispatch to its server directly",
                path
            )),
            None => Err(format!(
                "Catalog path '{}' not found on this server. Available top-level entries: {:?}",
                path,
                catalog.keys().collect::<Vec<_>>()
            )),
        },
        None => resolve_default_entry(catalog).ok_or_else(|| {
            format!(
                "No unambiguous default on this server — send a catalog path. Available top-level entries: {:?}",
                catalog.keys().collect::<Vec<_>>()
            )
        }),
    }
}

/// The catalog's own default render target, honored only when UNAMBIGUOUS:
/// an entry named "default", or a catalog with exactly one (source-bearing)
/// entry. A multi-entry catalog without a "default" entry has no honest
/// answer — the old behavior of picking the first source by declaration
/// order handed callers an arbitrary archetype.
fn resolve_default_entry(
    catalog: &LinkedHashMap<String, CatalogEntry>,
) -> Option<(CatalogEntry, String)> {
    if let Some(entry) = catalog.get("default") {
        return Some((entry.clone(), "default".to_string()));
    }
    if catalog.len() == 1 {
        let (name, entry) = catalog.iter().next()?;
        if entry.source.is_some() {
            return Some((entry.clone(), name.clone()));
        }
    }
    None
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
    fn default_entry_wins_by_name() {
        let mut root = LinkedHashMap::new();
        let (n, e) = leaf("default", "git://example.com/default.git");
        root.insert(n, e);
        let (n, e) = leaf("other", "git://example.com/other.git");
        root.insert(n, e);

        let (entry, label) = resolve_default_entry(&root).expect("default resolves");
        assert_eq!(label, "default");
        assert_eq!(entry.source.as_deref(), Some("git://example.com/default.git"));
    }

    #[test]
    fn single_leaf_catalog_is_unambiguous_whatever_its_name() {
        let mut root = LinkedHashMap::new();
        let (n, e) = leaf("solo", "git://example.com/solo.git");
        root.insert(n, e);

        let (entry, label) = resolve_default_entry(&root).expect("single leaf resolves");
        assert_eq!(label, "solo");
        assert_eq!(entry.source.as_deref(), Some("git://example.com/solo.git"));
    }

    #[test]
    fn multi_entry_catalog_without_default_is_ambiguous() {
        let mut root = LinkedHashMap::new();
        let (n, e) = leaf("alpha", "git://example.com/alpha.git");
        root.insert(n, e);
        let (n, e) = leaf("beta", "git://example.com/beta.git");
        root.insert(n, e);

        // The old behavior rendered `alpha` here (first source by declaration
        // order) — the arbitrary-entry bug this helper exists to kill.
        assert!(resolve_default_entry(&root).is_none());
    }

    #[test]
    fn single_group_catalog_is_not_a_render_target() {
        let mut children = LinkedHashMap::new();
        let (n, e) = leaf("grpc", "git://example.com/grpc.git");
        children.insert(n, e);
        let mut root = LinkedHashMap::new();
        let (n, e) = group("services", children);
        root.insert(n, e);

        assert!(resolve_default_entry(&root).is_none());
    }
}
