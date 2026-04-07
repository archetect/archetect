use std::fs;
use std::path::Path;

use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use archetect_api::{ClientMessage, ScriptMessage};

use crate::prompt_envelope::{LogEntry, PromptEnvelope, PromptType};

/// Result of draining messages from the render thread until a prompt or completion.
pub struct DrainResult {
    pub logs: Vec<LogEntry>,
    pub files_written: Vec<String>,
    pub outcome: DrainOutcome,
}

pub enum DrainOutcome {
    Prompt(PromptEnvelope),
    Complete { success: bool, message: Option<String> },
}

/// The state of the current render session.
pub enum SessionState {
    /// No render in progress.
    Idle,
    /// Render thread running, waiting for agent to respond to a prompt.
    Prompting {
        pending_prompt: PromptEnvelope,
        client_tx: mpsc::Sender<ClientMessage>,
        script_rx: mpsc::Receiver<ScriptMessage>,
        #[allow(dead_code)]
        render_handle: JoinHandle<()>,
    },
}

impl std::fmt::Debug for SessionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionState::Idle => write!(f, "Idle"),
            SessionState::Prompting { .. } => write!(f, "Prompting"),
        }
    }
}

impl SessionState {
    pub fn is_idle(&self) -> bool {
        matches!(self, SessionState::Idle)
    }
}

/// Drain messages from the render thread until we hit a prompt or completion.
/// Auto-Acks WriteFile/WriteDirectory and accumulates logs.
pub async fn drain_until_prompt_or_complete(
    script_rx: &mut mpsc::Receiver<ScriptMessage>,
    client_tx: &mpsc::Sender<ClientMessage>,
) -> Result<DrainResult, String> {
    let mut logs = Vec::new();
    let mut files_written = Vec::new();

    loop {
        match script_rx.recv().await {
            Some(ScriptMessage::WriteFile(info)) => {
                // Write the file to disk
                let path = Path::new(&info.destination);
                if let Some(parent) = path.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                let write_result = fs::write(path, &info.contents);
                files_written.push(info.destination.clone());
                let response = match write_result {
                    Ok(()) => ClientMessage::Ack,
                    Err(e) => ClientMessage::Error(format!("Failed to write {}: {}", info.destination, e)),
                };
                client_tx.send(response).await
                    .map_err(|_| "Render thread died while sending Ack".to_string())?;
            }
            Some(ScriptMessage::WriteDirectory(info)) => {
                // Create the directory
                let _ = fs::create_dir_all(&info.path);
                files_written.push(info.path.clone());
                client_tx.send(ClientMessage::Ack).await
                    .map_err(|_| "Render thread died while sending Ack".to_string())?;
            }
            Some(ScriptMessage::CompleteSuccess) => {
                return Ok(DrainResult {
                    logs,
                    files_written,
                    outcome: DrainOutcome::Complete { success: true, message: None },
                });
            }
            Some(ScriptMessage::CompleteError(msg)) => {
                return Ok(DrainResult {
                    logs,
                    files_written,
                    outcome: DrainOutcome::Complete { success: false, message: Some(msg) },
                });
            }
            Some(ref msg) if PromptEnvelope::from_script_message(msg).is_some() => {
                let envelope = PromptEnvelope::from_script_message(msg).unwrap();
                return Ok(DrainResult {
                    logs,
                    files_written,
                    outcome: DrainOutcome::Prompt(envelope),
                });
            }
            Some(ref msg) if LogEntry::from_script_message(msg).is_some() => {
                logs.push(LogEntry::from_script_message(msg).unwrap());
            }
            Some(_) => {
                // Unknown message type, skip
            }
            None => {
                return Err("Render thread exited unexpectedly".to_string());
            }
        }
    }
}

/// Convert a JSON value to the appropriate ClientMessage based on the prompt type.
pub fn json_to_client_message(
    value: &serde_json::Value,
    prompt_type: &PromptType,
) -> Result<ClientMessage, String> {
    match prompt_type {
        PromptType::Text | PromptType::Editor | PromptType::Select => {
            match value {
                serde_json::Value::String(s) => Ok(ClientMessage::String(s.clone())),
                serde_json::Value::Null => Ok(ClientMessage::None),
                other => Err(format!("Expected a string value, got {}", other)),
            }
        }
        PromptType::Int => {
            match value {
                serde_json::Value::Number(n) => {
                    n.as_i64()
                        .map(ClientMessage::Integer)
                        .ok_or_else(|| format!("Expected an integer, got {}", n))
                }
                serde_json::Value::String(s) => {
                    s.parse::<i64>()
                        .map(ClientMessage::Integer)
                        .map_err(|_| format!("Expected an integer, got '{}'", s))
                }
                serde_json::Value::Null => Ok(ClientMessage::None),
                other => Err(format!("Expected an integer value, got {}", other)),
            }
        }
        PromptType::Bool => {
            match value {
                serde_json::Value::Bool(b) => Ok(ClientMessage::Boolean(*b)),
                serde_json::Value::Null => Ok(ClientMessage::None),
                other => Err(format!("Expected a boolean value, got {}", other)),
            }
        }
        PromptType::MultiSelect | PromptType::List => {
            match value {
                serde_json::Value::Array(arr) => {
                    let strings: Result<Vec<String>, _> = arr.iter().map(|v| {
                        v.as_str()
                            .map(String::from)
                            .ok_or_else(|| format!("Array items must be strings, got {}", v))
                    }).collect();
                    strings.map(ClientMessage::Array)
                }
                serde_json::Value::Null => Ok(ClientMessage::None),
                other => Err(format!("Expected an array of strings, got {}", other)),
            }
        }
    }
}
