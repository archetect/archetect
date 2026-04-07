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
use archetect_core::catalog::Catalog;
use archetect_core::source::SourceContents;
use archetect_core::Archetect;

use crate::io_handle::McpScriptIoHandle;
use crate::prompt_envelope::ToolResponse;
use crate::session::{
    drain_until_prompt_or_complete, json_to_client_message, DrainOutcome, SessionState,
};

#[derive(Debug, Clone)]
pub struct ArchetectMcpServer {
    archetect: Archetect,
    session: Arc<Mutex<SessionState>>,
    tool_router: ToolRouter<Self>,
}

impl ArchetectMcpServer {
    pub fn new(archetect: Archetect) -> Self {
        Self {
            archetect,
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
        let mut answers = rhai::Map::new();
        if let Some(answers_json) = &req.answers {
            if let serde_json::Value::Object(obj) = answers_json {
                for (k, v) in obj {
                    if let Ok(dynamic) = serde_json::from_value::<rhai::Dynamic>(v.clone()) {
                        answers.insert(k.clone().into(), dynamic);
                    }
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
            .with_layout(match archetect_core::system::RootedSystemLayout::dot_home() {
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
                    SourceContents::Catalog => {
                        let catalog = Catalog::load(archetect.clone(), source)
                            .map_err(|e| format!("Catalog error: {}", e))?;
                        catalog
                            .check_requirements()
                            .map_err(|e| format!("Requirements error: {}", e))?;
                        catalog
                            .render(render_context)
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
}

fn to_json(response: &ToolResponse) -> String {
    serde_json::to_string_pretty(response)
        .unwrap_or_else(|e| format!("{{\"status\": \"error\", \"message\": \"{}\"}}", e))
}
