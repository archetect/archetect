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
            Some(msg) => {
                if let Some(envelope) = PromptEnvelope::from_script_message(&msg) {
                    return Ok(DrainResult {
                        logs,
                        files_written,
                        outcome: DrainOutcome::Prompt(envelope),
                    });
                } else if let Some(entry) = LogEntry::from_script_message(&msg) {
                    logs.push(entry);
                }
                // Unknown message types fall through and are skipped.
            }
            None => {
                return Err("Render thread exited unexpectedly".to_string());
            }
        }
    }
}

/// Interpret a string a client sent where a list was expected. Tries a
/// JSON array first (stringifying clients commonly send the encoded
/// array), then falls back to comma-splitting with optional surrounding
/// brackets. An empty string is an empty list.
fn parse_string_as_list(s: &str) -> Vec<String> {
    if let Ok(items) = serde_json::from_str::<Vec<String>>(s) {
        return items;
    }
    let trimmed = s.trim();
    let inner = trimmed
        .strip_prefix('[')
        .and_then(|rest| rest.strip_suffix(']'))
        .unwrap_or(trimmed);
    inner
        .split(',')
        .map(|item| item.trim().trim_matches(|c| c == '"' || c == '\'').to_string())
        .filter(|item| !item.is_empty())
        .collect()
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
                // Some MCP clients stringify every value (the `value` schema is
                // untyped) — accept the string forms rather than dead-ending
                // the session on a prompt the agent cannot answer any other way.
                serde_json::Value::String(s) => match s.trim().to_lowercase().as_str() {
                    "true" | "yes" => Ok(ClientMessage::Boolean(true)),
                    "false" | "no" => Ok(ClientMessage::Boolean(false)),
                    _ => Err(format!("Expected a boolean, got '{}'", s)),
                },
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
                // Stringifying clients again: accept a JSON-encoded array
                // (`"[\"a\",\"b\"]"`) or a comma-separated list (`"a,b"`,
                // `"[a, b]"`) — the same shapes `-a key=[a,b]` accepts on
                // the CLI.
                serde_json::Value::String(s) => Ok(ClientMessage::Array(parse_string_as_list(s))),
                serde_json::Value::Null => Ok(ClientMessage::None),
                other => Err(format!("Expected an array of strings, got {}", other)),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn to_array(value: serde_json::Value, prompt_type: PromptType) -> Vec<String> {
        match json_to_client_message(&value, &prompt_type) {
            Ok(ClientMessage::Array(items)) => items,
            other => panic!("expected Array, got {:?}", other.map(|_| "non-array ok")),
        }
    }

    #[test]
    fn multiselect_accepts_json_encoded_array_string() {
        let value = serde_json::Value::String(r#"["metrics","health"]"#.to_string());
        assert_eq!(to_array(value, PromptType::MultiSelect), vec!["metrics", "health"]);
    }

    #[test]
    fn list_accepts_comma_separated_string() {
        let value = serde_json::Value::String("a, b ,c".to_string());
        assert_eq!(to_array(value, PromptType::List), vec!["a", "b", "c"]);
    }

    #[test]
    fn multiselect_accepts_bracketed_unquoted_string() {
        let value = serde_json::Value::String("[metrics, health]".to_string());
        assert_eq!(to_array(value, PromptType::MultiSelect), vec!["metrics", "health"]);
    }

    #[test]
    fn multiselect_empty_string_is_empty_list() {
        let value = serde_json::Value::String("".to_string());
        assert_eq!(to_array(value, PromptType::MultiSelect), Vec::<String>::new());
    }

    #[test]
    fn bool_accepts_string_forms() {
        for (input, expected) in [("true", true), ("False", false), ("yes", true), ("no", false)] {
            let value = serde_json::Value::String(input.to_string());
            match json_to_client_message(&value, &PromptType::Bool) {
                Ok(ClientMessage::Boolean(b)) => assert_eq!(b, expected, "input {input}"),
                other => panic!("expected Boolean for {input}, got {:?}", other.map(|_| "ok")),
            }
        }
    }

    #[test]
    fn bool_rejects_non_boolean_string() {
        assert!(json_to_client_message(
            &serde_json::Value::String("maybe".to_string()),
            &PromptType::Bool
        )
        .is_err());
    }
}
