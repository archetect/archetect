use std::collections::HashSet;
use std::sync::Arc;

use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_handler, tool_router, ServerHandler};
use schemars::JsonSchema;
use serde::Deserialize;
use tokio::sync::Mutex;

use archetect_api::ClientMessage;
use archetect_core::archetype::archetype::Archetype;
use archetect_core::archetype::render_context::RenderContext;
use archetect_core::catalog::{CatalogIndex, CatalogIndexer};
use archetect_core::source::SourceContents;
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
}

#[tool_handler]
impl ServerHandler for ArchetectMcpServer {}

#[derive(Deserialize, JsonSchema)]
pub struct RenderRequest {
    /// Archetype source — a Git URL or local filesystem path
    pub source: String,
    /// Destination directory for rendered output
    pub destination: String,
    /// Pre-supplied answers as a JSON object (key-value pairs) to skip prompts
    pub answers: Option<serde_json::Value>,
    /// Switches to enable (list of switch names)
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
    pub answers: Option<serde_json::Value>,
    /// Switches to enable (list of switch names)
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

#[tool_router]
impl ArchetectMcpServer {
    #[tool(
        name = "render",
        description = "Render a project from an archetype template. Starts a stateful render session. Returns the first prompt (if any) or completion status. Use the 'respond' tool to answer prompts."
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
        if let Some(answers_json) = &req.answers {
            if let serde_json::Value::Object(obj) = answers_json {
                for (k, v) in obj {
                    answers.insert(k.clone(), archetect_api::ContextValue::from(v.clone()));
                }
            }
        }

        // Build render context
        let mut render_context = RenderContext::new(
            camino::Utf8PathBuf::from(&req.destination),
            answers,
        );
        if let Some(switches) = &req.switches {
            let switch_set: HashSet<String> = switches.iter().cloned().collect();
            render_context = render_context.with_switches(switch_set);
        }
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
        description = "Render an archetype by its catalog path (e.g. 'services/grpc'). Resolves the path in the catalog, applies any pre-configured answers and switches from the catalog entry, and starts a render session. Use 'respond' to answer prompts."
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

        // Resolve the catalog path to find the source and pre-configured settings
        let catalog = match self.archetect.configuration().catalog() {
            Some(c) => c,
            None => {
                return to_json(&ToolResponse::error("No catalog configured"));
            }
        };

        let entry = match archetect_core::catalog::resolve_path(catalog, &req.path) {
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

        // Build render context — merge catalog-level and request-level settings
        let mut answers = archetect_api::ContextMap::new();

        // Catalog-entry-level answers first
        if let Some(ref entry_answers) = entry.answers {
            for (k, v) in entry_answers {
                answers.insert(k.clone(), v.clone());
            }
        }
        // Request-level answers override
        if let Some(answers_json) = &req.answers {
            if let serde_json::Value::Object(obj) = answers_json {
                for (k, v) in obj {
                    answers.insert(k.clone(), archetect_api::ContextValue::from(v.clone()));
                }
            }
        }

        let mut render_context = RenderContext::new(
            camino::Utf8PathBuf::from(&req.destination),
            answers,
        );

        // Merge switches: catalog-entry + request
        let mut switch_set = entry.switches.clone().unwrap_or_default();
        if let Some(req_switches) = &req.switches {
            switch_set.extend(req_switches.iter().cloned());
        }
        if !switch_set.is_empty() {
            render_context = render_context.with_switches(switch_set);
        }

        // Merge defaults
        if let Some(ref use_defaults) = entry.use_defaults {
            render_context.set_use_defaults(use_defaults.clone());
        }
        if entry.use_defaults_all.unwrap_or(false) || req.use_defaults_all.unwrap_or(false) {
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
