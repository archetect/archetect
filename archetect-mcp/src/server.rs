use std::collections::HashSet;
use std::sync::Arc;

use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::AnnotateAble;
use rmcp::{tool, tool_handler, tool_router, ServerHandler};
use schemars::JsonSchema;
use serde::Deserialize;
use tokio::sync::Mutex;

use archetect_api::ClientMessage;
use archetect_core::archetype::archetype::Archetype;
use archetect_core::archetype::render_context::RenderContext;
use archetect_core::catalog::{CatalogIndex, CatalogIndexer};
use archetect_core::source::SourceContents;
use archetect_core::{help, learn};
use archetect_core::Archetect;

use crate::io_handle::McpScriptIoHandle;
use crate::prompt_envelope::{
    CatalogBrowseResponse, CatalogEntryInfo, CatalogSearchResponse, ToolResponse,
};
use crate::session::{
    drain_until_prompt_or_complete, json_to_client_message, DrainOutcome, SessionState,
};

#[derive(Debug, Clone)]
pub struct ArchetectMcpServer {
    archetect: Archetect,
    catalog_index: CatalogIndex,
    session: Arc<Mutex<SessionState>>,
    // Read through the `#[tool_handler]` macro on the ServerHandler
    // impl — dead-code analysis can't see past the macro expansion.
    #[allow(dead_code)]
    tool_router: ToolRouter<Self>,
}

impl ArchetectMcpServer {
    pub fn new(archetect: Archetect) -> Self {
        // Build a deep catalog index by recursively resolving all sources.
        let catalog_index = match archetect.configuration().catalog() {
            Some(catalog) => {
                CatalogIndexer::new(archetect.clone()).build_index(catalog)
            }
            None => CatalogIndex::from_entries(Vec::new()),
        };

        Self {
            archetect,
            catalog_index,
            session: Arc::new(Mutex::new(SessionState::Idle)),
            tool_router: Self::tool_router(),
        }
    }

    /// The learn renderer's environment facts, computed fresh per ask so the answer is true at
    /// the moment of asking (the catalog INDEX is startup-frozen; these cheap facts need not be).
    fn learn_env(&self) -> learn::RenderEnv {
        learn::RenderEnv::from_configuration(
            self.archetect.configuration(),
            self.archetect.layout().as_ref(),
        )
    }
}

#[tool_handler]
impl ServerHandler for ArchetectMcpServer {
    /// The server's identity: the embedded skill arrives as `instructions`, so a connected
    /// agent starts knowing the loop instead of six bare tool names.
    fn get_info(&self) -> rmcp::model::ServerInfo {
        rmcp::model::ServerInfo::new(
            rmcp::model::ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .build(),
        )
        .with_instructions(learn::SKILL)
    }

    /// The learn topics, protocol-native: `archetect://learn/<topic>` + `archetect://skill` —
    /// the same content the `learn` tool serves, for clients that prefetch resources.
    fn list_resources(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParams>,
        _context: rmcp::service::RequestContext<rmcp::service::RoleServer>,
    ) -> impl std::future::Future<Output = Result<rmcp::model::ListResourcesResult, rmcp::ErrorData>> + '_
    {
        let mut resources: Vec<rmcp::model::Resource> = vec![rmcp::model::RawResource {
            uri: "archetect://skill".into(),
            name: "skill".into(),
            title: Some("The Archetect agent skill".into()),
            description: Some("The practice: render, don't hand-write — and how to drive archetect".into()),
            mime_type: Some("text/markdown".into()),
            size: None,
            icons: None,
            meta: None,
        }
        .no_annotation()];
        for topic in learn::Topic::ALL {
            resources.push(
                rmcp::model::RawResource {
                    uri: format!("archetect://learn/{}", topic.key()),
                    name: topic.key().into(),
                    title: Some(format!("learn: {}", topic.key())),
                    description: Some(topic.hook().into()),
                    mime_type: Some("text/markdown".into()),
                    size: None,
                    icons: None,
                    meta: None,
                }
                .no_annotation(),
            );
        }
        std::future::ready(Ok(rmcp::model::ListResourcesResult::with_all_items(resources)))
    }

    fn read_resource(
        &self,
        request: rmcp::model::ReadResourceRequestParams,
        _context: rmcp::service::RequestContext<rmcp::service::RoleServer>,
    ) -> impl std::future::Future<Output = Result<rmcp::model::ReadResourceResult, rmcp::ErrorData>> + '_
    {
        let uri = request.uri.clone();
        let answer = if uri == "archetect://skill" {
            Ok(learn::SKILL.to_string())
        } else if let Some(topic) = uri.strip_prefix("archetect://learn/") {
            let env = self.learn_env();
            learn::answer(Some(topic), &env, learn::Transport::Mcp)
                .map_err(|e| format!("{uri}: {e}"))
        } else {
            Err(format!(
                "unknown resource {uri:?} — this server serves archetect://skill and archetect://learn/<topic>"
            ))
        };
        std::future::ready(match answer {
            Ok(text) => Ok(rmcp::model::ReadResourceResult::new(vec![
                rmcp::model::ResourceContents::text(text, uri),
            ])),
            Err(message) => Err(rmcp::ErrorData::resource_not_found(message, None)),
        })
    }
}

#[derive(Deserialize, JsonSchema)]
pub struct RenderRequest {
    /// Archetype source — a Git URL or local filesystem path
    pub source: String,
    /// Destination directory for rendered output
    pub destination: String,
    /// Pre-supplied answers as a JSON object (key-value pairs) to skip prompts
    pub answers: Option<std::collections::HashMap<String, serde_json::Value>>,
    /// Switches to enable (list of switch names). Switches are boolean flags
    /// that control hidden archetype behaviour — they are NOT prompted for
    /// during execution and must be set here, before the session starts.
    /// Check the archetype's interface.yaml for available switches.
    pub switches: Option<Vec<String>>,
    /// Use default values for all prompts that have defaults
    pub use_defaults_all: Option<bool>,
}

#[derive(Deserialize, JsonSchema)]
pub struct CatalogBrowseRequest {
    /// Catalog path to browse. Omit or leave empty for root entries.
    /// Use slash-separated paths like "services/rust" to browse deeper.
    pub path: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct CatalogSearchRequest {
    /// Search query — space-separated terms. All terms must match (AND semantics).
    /// Searches across entry names, descriptions, paths, languages, frameworks, and tags.
    pub query: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct CatalogRenderRequest {
    /// Catalog path of the archetype to render (e.g. "services/grpc")
    pub path: String,
    /// Destination directory for rendered output
    pub destination: String,
    /// Pre-supplied answers as a JSON object (key-value pairs) to skip prompts
    pub answers: Option<std::collections::HashMap<String, serde_json::Value>>,
    /// Switches to enable (list of switch names). Switches are boolean flags
    /// that control hidden archetype behaviour — they are NOT prompted for
    /// during execution and must be set here, before the session starts.
    /// Check the archetype's interface.yaml for available switches.
    pub switches: Option<Vec<String>>,
    /// Use default values for all prompts that have defaults
    pub use_defaults_all: Option<bool>,
}

#[derive(Deserialize, JsonSchema)]
pub struct RespondRequest {
    /// Response value matching the prompt type: string for text/editor/select,
    /// integer for int, boolean for bool, array of strings for list/multiselect.
    /// Use null to skip an optional prompt.
    pub value: serde_json::Value,
}

#[derive(Deserialize, JsonSchema)]
pub struct LearnRequest {
    /// Topic key or alias (e.g. "authoring", "templates", "atl"). Omit to list every topic.
    pub topic: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct IntrospectRequest {
    /// Case-insensitive substring filter over API names and summaries (e.g. "prompt", "case").
    /// Omit for the whole surface.
    pub filter: Option<String>,
}

#[tool_router]
impl ArchetectMcpServer {
    #[tool(
        name = "learn",
        description = "Learn Archetect from the binary: progressive-disclosure topics, one screen each, computed for THIS environment (catalog, cache, locals). Call with no topic to list topics; aliases resolve (atl → templates). Returns markdown. Topics are also served as resources (archetect://learn/<topic>)."
    )]
    async fn learn(&self, Parameters(req): Parameters<LearnRequest>) -> String {
        let env = self.learn_env();
        match learn::answer(req.topic.as_deref(), &env, learn::Transport::Mcp) {
            Ok(text) => text,
            Err(message) => format!("learn: {message}"),
        }
    }

    #[tool(
        name = "introspect",
        description = "The scripting API's shapes — every Context method, prompt option, module function, and class field, computed from the embedded annotations. Filter narrows by substring over names and summaries. Returns compact JSON { entries: [{ name, signature, summary }] }. Never guess an API shape: ask this."
    )]
    async fn introspect(&self, Parameters(req): Parameters<IntrospectRequest>) -> String {
        let entries = help::core_entries();
        let entries = match req.filter.as_deref() {
            Some(needle) => help::filter(&entries, needle),
            None => entries,
        };
        let items: Vec<serde_json::Value> = entries
            .iter()
            .map(|e| {
                serde_json::json!({
                    "name": e.name,
                    "signature": e.signature,
                    "summary": e.summary,
                })
            })
            .collect();
        serde_json::json!({ "entries": items }).to_string()
    }

    #[tool(
        name = "render",
        description = "Render a project from an archetype template. Starts a stateful render session. Returns the first prompt (if any) or completion status. Use the 'respond' tool to answer prompts. Switches must be set up front in this call — they are not prompted for during the session. Check the archetype's interface.yaml for available switches and prompts."
    )]
    async fn render(
        &self,
        Parameters(req): Parameters<RenderRequest>,
    ) -> String {
        let mut session = self.session.lock().await;

        if !session.is_idle() {
            return to_json(&ToolResponse::error(
                "A render session is already active. Use 'cancel' to abort it first.",
            ));
        }

        // Parse answers
        let mut answers = archetect_api::ContextMap::new();
        if let Some(obj) = &req.answers {
            for (k, v) in obj {
                answers.insert(k.clone(), archetect_api::ContextValue::from(v.clone()));
            }
        }

        // Build render context
        let mut render_context = RenderContext::new(
            camino::Utf8PathBuf::from(&req.destination),
            answers,
        );
        // Base switches come from configuration; request tokens overlay them
        // per-item (`name` adds, `name=false` removes).
        let mut switch_set: HashSet<String> =
            self.archetect.configuration().switches().iter().cloned().collect();
        if let Some(switches) = &req.switches {
            if let Err(e) = archetect_core::flags::overlay_flag_tokens(
                &mut switch_set,
                switches.iter().map(String::as_str),
                "switch",
                "render request",
            ) {
                return to_json(&ToolResponse::error(e.to_string()));
            }
        }
        render_context = render_context.with_switches(switch_set);
        if req.use_defaults_all.unwrap_or(false) {
            render_context = render_context.with_use_defaults_all(true);
        }

        // Create IO channels
        let (script_tx, mut script_rx) = tokio::sync::mpsc::channel(1);
        let (client_tx, client_rx) = tokio::sync::mpsc::channel(1);

        let io_handle = McpScriptIoHandle::new(script_tx, client_rx);

        // Build a new Archetect with the MCP IO handle
        let archetect = match Archetect::builder()
            .with_driver(io_handle)
            .with_configuration(self.archetect.configuration().clone())
            .with_layout(match archetect_core::system::XdgSystemLayout::new() {
                Ok(layout) => layout,
                Err(e) => {
                    return to_json(&ToolResponse::error(format!("Layout error: {}", e)));
                }
            })
            .build()
        {
            Ok(a) => a,
            Err(e) => {
                return to_json(&ToolResponse::error(format!("Failed to initialize: {}", e)));
            }
        };

        // Resolve source and spawn render
        let source_str = req.source.clone();
        let render_handle = tokio::task::spawn_blocking(move || {
            let result = (|| -> Result<(), String> {
                let source = archetect
                    .new_source(&source_str)
                    .map_err(|e| format!("Source error: {}", e))?;

                match source.source_contents() {
                    SourceContents::Archetype => {
                        let archetype = Archetype::new(archetect.clone(), source)
                            .map_err(|e| format!("Archetype error: {}", e))?;
                        archetype
                            .check_requirements()
                            .map_err(|e| format!("Requirements error: {}", e))?;
                        archetype
                            .render(render_context)
                            .map(|_| ())
                            .map_err(|e| format!("Render error: {}", e))?;
                    }
                    SourceContents::Unknown => {
                        return Err("Unknown source type".to_string());
                    }
                }
                Ok(())
            })();

            match result {
                Ok(()) => {
                    let _ = archetect.request(archetect_api::ScriptMessage::CompleteSuccess);
                }
                Err(e) => {
                    let _ = archetect.request(archetect_api::ScriptMessage::CompleteError(e));
                }
            }
        });

        // Drain until first prompt or completion
        let drain_result = match drain_until_prompt_or_complete(&mut script_rx, &client_tx).await {
            Ok(r) => r,
            Err(e) => {
                return to_json(&ToolResponse::error(e));
            }
        };

        match drain_result.outcome {
            DrainOutcome::Prompt(envelope) => {
                let response = ToolResponse::prompting(
                    drain_result.logs,
                    drain_result.files_written,
                    envelope.clone(),
                );
                *session = SessionState::Prompting {
                    pending_prompt: envelope,
                    client_tx,
                    script_rx,
                    render_handle,
                };
                to_json(&response)
            }
            DrainOutcome::Complete { success, message } => {
                *session = SessionState::Idle;
                if success {
                    to_json(&ToolResponse::complete(
                        drain_result.logs,
                        drain_result.files_written,
                    ))
                } else {
                    to_json(&ToolResponse::error(
                        message.unwrap_or_else(|| "Render failed".into()),
                    ))
                }
            }
        }
    }

    #[tool(
        name = "respond",
        description = "Respond to the current prompt in an active render session. The value type must match the prompt type shown in the previous response."
    )]
    async fn respond(
        &self,
        Parameters(req): Parameters<RespondRequest>,
    ) -> String {
        let mut session = self.session.lock().await;

        let (pending_prompt, client_tx, mut script_rx, render_handle) =
            match std::mem::replace(&mut *session, SessionState::Idle) {
                SessionState::Prompting {
                    pending_prompt,
                    client_tx,
                    script_rx,
                    render_handle,
                } => (pending_prompt, client_tx, script_rx, render_handle),
                SessionState::Idle => {
                    return to_json(&ToolResponse::error(
                        "No active render session. Use 'render' to start one.",
                    ));
                }
            };

        // Convert JSON value to ClientMessage based on prompt type
        let client_msg = match json_to_client_message(&req.value, &pending_prompt.prompt_type) {
            Ok(msg) => msg,
            Err(e) => {
                // Put session back
                *session = SessionState::Prompting {
                    pending_prompt,
                    client_tx,
                    script_rx,
                    render_handle,
                };
                return to_json(&ToolResponse::error(format!("Invalid response: {}", e)));
            }
        };

        // Send response to render thread
        if client_tx.send(client_msg).await.is_err() {
            *session = SessionState::Idle;
            return to_json(&ToolResponse::error("Render thread died"));
        }

        // Drain until next prompt or completion
        let drain_result = match drain_until_prompt_or_complete(&mut script_rx, &client_tx).await {
            Ok(r) => r,
            Err(e) => {
                *session = SessionState::Idle;
                return to_json(&ToolResponse::error(e));
            }
        };

        match drain_result.outcome {
            DrainOutcome::Prompt(envelope) => {
                let response = ToolResponse::prompting(
                    drain_result.logs,
                    drain_result.files_written,
                    envelope.clone(),
                );
                *session = SessionState::Prompting {
                    pending_prompt: envelope,
                    client_tx,
                    script_rx,
                    render_handle,
                };
                to_json(&response)
            }
            DrainOutcome::Complete { success, message } => {
                *session = SessionState::Idle;
                if success {
                    to_json(&ToolResponse::complete(
                        drain_result.logs,
                        drain_result.files_written,
                    ))
                } else {
                    to_json(&ToolResponse::error(
                        message.unwrap_or_else(|| "Render failed".into()),
                    ))
                }
            }
        }
    }

    #[tool(
        name = "cancel",
        description = "Cancel the current render session."
    )]
    async fn cancel(&self) -> String {
        let mut session = self.session.lock().await;

        match std::mem::replace(&mut *session, SessionState::Idle) {
            SessionState::Prompting { client_tx, .. } => {
                let _ = client_tx.send(ClientMessage::Abort).await;
                to_json(&ToolResponse::cancelled())
            }
            SessionState::Idle => {
                to_json(&ToolResponse::error("No active render session to cancel"))
            }
        }
    }

    #[tool(
        name = "catalog_browse",
        description = "Browse the archetype catalog tree. Returns entries at the given path. Omit path for root entries. Groups contain children; leaves are renderable archetypes."
    )]
    async fn catalog_browse(
        &self,
        Parameters(req): Parameters<CatalogBrowseRequest>,
    ) -> String {
        let path = req.path.as_deref().unwrap_or("");

        if path.is_empty() {
            // Root entries
            let entries: Vec<CatalogEntryInfo> = self
                .catalog_index
                .root()
                .iter()
                .map(CatalogEntryInfo::from_index_entry)
                .collect();
            return to_json_generic(&CatalogBrowseResponse {
                path: String::new(),
                entries,
            });
        }

        // Check if path resolves to a specific entry
        match self.catalog_index.get(path) {
            Some(entry) => {
                if entry.children.is_empty() {
                    // Leaf or empty group — return single entry info
                    to_json_generic(&CatalogBrowseResponse {
                        path: path.to_owned(),
                        entries: vec![CatalogEntryInfo::from_index_entry(entry)],
                    })
                } else {
                    // Group — return children
                    let entries: Vec<CatalogEntryInfo> = entry
                        .children
                        .iter()
                        .map(CatalogEntryInfo::from_index_entry)
                        .collect();
                    to_json_generic(&CatalogBrowseResponse {
                        path: path.to_owned(),
                        entries,
                    })
                }
            }
            None => {
                to_json(&ToolResponse::error(format!(
                    "Catalog path '{}' not found",
                    path
                )))
            }
        }
    }

    #[tool(
        name = "catalog_search",
        description = "Search the archetype catalog. Returns entries whose name, description, path, languages, frameworks, or tags match all query terms (AND semantics). Use this to discover available archetypes."
    )]
    async fn catalog_search(
        &self,
        Parameters(req): Parameters<CatalogSearchRequest>,
    ) -> String {
        let results: Vec<CatalogEntryInfo> = self
            .catalog_index
            .search(&req.query)
            .into_iter()
            .map(CatalogEntryInfo::from_index_entry)
            .collect();

        to_json_generic(&CatalogSearchResponse {
            query: req.query,
            results,
        })
    }

    #[tool(
        name = "catalog_render",
        description = "Render an archetype by its catalog path (e.g. 'services/grpc'). Resolves the path in the catalog, applies any pre-configured answers and switches from the catalog entry, and starts a render session. Use 'respond' to answer prompts. Switches must be set up front in this call — they are not prompted for during the session."
    )]
    async fn catalog_render(
        &self,
        Parameters(req): Parameters<CatalogRenderRequest>,
    ) -> String {
        let mut session = self.session.lock().await;

        if !session.is_idle() {
            return to_json(&ToolResponse::error(
                "A render session is already active. Use 'cancel' to abort it first.",
            ));
        }

        // Resolve the catalog path via the catalog index. The index walks
        // sub-catalogs (unlike the manifest-only `resolve_path`), so nested
        // paths like `archetect/common/starters/archetype-starter` work.
        let entry = match self.catalog_index.get(&req.path) {
            Some(e) => e,
            None => {
                return to_json(&ToolResponse::error(format!(
                    "Catalog path '{}' not found",
                    req.path
                )));
            }
        };

        let source_str = match &entry.source {
            Some(s) => s.clone(),
            None => {
                return to_json(&ToolResponse::error(format!(
                    "Catalog entry '{}' has no source (it may be a group — try browsing it instead)",
                    req.path
                )));
            }
        };

        // Build render context — request-level answers only for now. The
        // catalog index does not yet carry entry-level answers/switches;
        // when it does, merge them here (catalog first, request overrides).
        let mut answers = archetect_api::ContextMap::new();
        if let Some(obj) = &req.answers {
            for (k, v) in obj {
                answers.insert(k.clone(), archetect_api::ContextValue::from(v.clone()));
            }
        }

        let mut render_context = RenderContext::new(
            camino::Utf8PathBuf::from(&req.destination),
            answers,
        );

        // Base switches come from configuration; request tokens overlay them
        // per-item (`name` adds, `name=false` removes).
        let mut switch_set: HashSet<String> =
            self.archetect.configuration().switches().iter().cloned().collect();
        if let Some(req_switches) = &req.switches {
            if let Err(e) = archetect_core::flags::overlay_flag_tokens(
                &mut switch_set,
                req_switches.iter().map(String::as_str),
                "switch",
                "render request",
            ) {
                return to_json(&ToolResponse::error(e.to_string()));
            }
        }
        render_context = render_context.with_switches(switch_set);

        // Defaults: request-level only for now (see note above).
        if req.use_defaults_all.unwrap_or(false) {
            render_context = render_context.with_use_defaults_all(true);
        }

        // Create IO channels and spawn render — same flow as the `render` tool
        let (script_tx, mut script_rx) = tokio::sync::mpsc::channel(1);
        let (client_tx, client_rx) = tokio::sync::mpsc::channel(1);

        let io_handle = McpScriptIoHandle::new(script_tx, client_rx);

        let archetect = match Archetect::builder()
            .with_driver(io_handle)
            .with_configuration(self.archetect.configuration().clone())
            .with_layout(match archetect_core::system::XdgSystemLayout::new() {
                Ok(layout) => layout,
                Err(e) => {
                    return to_json(&ToolResponse::error(format!("Layout error: {}", e)));
                }
            })
            .build()
        {
            Ok(a) => a,
            Err(e) => {
                return to_json(&ToolResponse::error(format!("Failed to initialize: {}", e)));
            }
        };

        let render_handle = tokio::task::spawn_blocking(move || {
            let result = (|| -> Result<(), String> {
                let source = archetect
                    .new_source(&source_str)
                    .map_err(|e| format!("Source error: {}", e))?;

                match source.source_contents() {
                    SourceContents::Archetype => {
                        let archetype = Archetype::new(archetect.clone(), source)
                            .map_err(|e| format!("Archetype error: {}", e))?;
                        archetype
                            .check_requirements()
                            .map_err(|e| format!("Requirements error: {}", e))?;
                        archetype
                            .render(render_context)
                            .map(|_| ())
                            .map_err(|e| format!("Render error: {}", e))?;
                    }
                    SourceContents::Unknown => {
                        return Err("Unknown source type".to_string());
                    }
                }
                Ok(())
            })();

            match result {
                Ok(()) => {
                    let _ = archetect.request(archetect_api::ScriptMessage::CompleteSuccess);
                }
                Err(e) => {
                    let _ = archetect.request(archetect_api::ScriptMessage::CompleteError(e));
                }
            }
        });

        // Drain until first prompt or completion
        let drain_result = match drain_until_prompt_or_complete(&mut script_rx, &client_tx).await {
            Ok(r) => r,
            Err(e) => {
                return to_json(&ToolResponse::error(e));
            }
        };

        match drain_result.outcome {
            DrainOutcome::Prompt(envelope) => {
                let response = ToolResponse::prompting(
                    drain_result.logs,
                    drain_result.files_written,
                    envelope.clone(),
                );
                *session = SessionState::Prompting {
                    pending_prompt: envelope,
                    client_tx,
                    script_rx,
                    render_handle,
                };
                to_json(&response)
            }
            DrainOutcome::Complete { success, message } => {
                *session = SessionState::Idle;
                if success {
                    to_json(&ToolResponse::complete(
                        drain_result.logs,
                        drain_result.files_written,
                    ))
                } else {
                    to_json(&ToolResponse::error(
                        message.unwrap_or_else(|| "Render failed".into()),
                    ))
                }
            }
        }
    }
}

fn to_json(response: &ToolResponse) -> String {
    serde_json::to_string_pretty(response)
        .unwrap_or_else(|e| format!("{{\"status\": \"error\", \"message\": \"{}\"}}", e))
}

fn to_json_generic<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_string_pretty(value)
        .unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e))
}
