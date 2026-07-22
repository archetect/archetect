//! The recording IO driver behind the interface probe.

use std::collections::{BTreeMap, VecDeque};
use std::sync::{Arc, Mutex};

use archetect_api::{
    ClientMessage, IoError, PromptEnvelope, ScriptIoHandle, ScriptMessage,
};

/// What one probe run recorded.
#[derive(Debug, Default)]
pub struct ProbeState {
    pub prompts: Vec<PromptEnvelope>,
    pub budget_hit: bool,
    queued: VecDeque<ClientMessage>,
}

/// A `ScriptIoHandle` that answers prompts instead of asking anyone:
/// override → default → type-synthetic. Writes are Ack'd and discarded;
/// every prompt's envelope is recorded. Fully synchronous — `send`
/// queues the reply `receive` will return, so the render runs inline on
/// the caller's thread with no channels.
#[derive(Debug)]
pub struct ProbeDriver {
    state: Arc<Mutex<ProbeState>>,
    prompt_budget: usize,
    overrides: BTreeMap<String, serde_json::Value>,
}

impl ProbeDriver {
    pub fn new(prompt_budget: usize, overrides: BTreeMap<String, serde_json::Value>) -> Self {
        ProbeDriver {
            state: Arc::new(Mutex::new(ProbeState::default())),
            prompt_budget,
            overrides,
        }
    }

    /// Shared handle to the recording — read it after the render returns.
    pub fn state(&self) -> Arc<Mutex<ProbeState>> {
        self.state.clone()
    }

    fn override_for(&self, key: Option<&str>) -> Option<ClientMessage> {
        let key = key?;
        let value = self.overrides.get(key)?;
        Some(match value {
            serde_json::Value::String(s) => ClientMessage::String(s.clone()),
            serde_json::Value::Bool(b) => ClientMessage::Boolean(*b),
            serde_json::Value::Number(n) => ClientMessage::Integer(n.as_i64().unwrap_or(0)),
            serde_json::Value::Array(items) => ClientMessage::Array(
                items
                    .iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect(),
            ),
            _ => return None,
        })
    }
}

/// The auto-answer: default first, else a type-appropriate synthetic
/// value; optional prompts with no default are skipped. Synthetics are
/// deliberately boring — the probe wants the transcript, not the output.
fn synthesize(msg: &ScriptMessage) -> Option<ClientMessage> {
    match msg {
        ScriptMessage::PromptForText(info) => Some(match info.default() {
            Some(default) => ClientMessage::String(default),
            None if info.optional => ClientMessage::None,
            // Lowercase-alpha survives the common identifier patterns;
            // a pattern it violates ends the run → coverage: partial.
            None => ClientMessage::String("probe".to_string()),
        }),
        ScriptMessage::PromptForInt(info) => Some(match info.default {
            Some(default) => ClientMessage::Integer(default),
            None if info.optional => ClientMessage::None,
            None => ClientMessage::Integer(info.min.unwrap_or(0).max(0).min(info.max.unwrap_or(i64::MAX))),
        }),
        ScriptMessage::PromptForBool(info) => Some(match info.default {
            Some(default) => ClientMessage::Boolean(default),
            None if info.optional => ClientMessage::None,
            None => ClientMessage::Boolean(false),
        }),
        ScriptMessage::PromptForSelect(info) => Some(match info.default() {
            Some(default) => ClientMessage::String(default),
            None => match info.options().first() {
                Some(first) => ClientMessage::String(first.value.clone()),
                None if info.optional => ClientMessage::None,
                None => ClientMessage::String(String::new()),
            },
        }),
        ScriptMessage::PromptForMultiSelect(info) => Some(match info.defaults() {
            Some(defaults) => ClientMessage::Array(defaults),
            None => {
                let need = info.min_items.unwrap_or(0);
                if need == 0 {
                    ClientMessage::Array(Vec::new())
                } else {
                    ClientMessage::Array(
                        info.options()
                            .iter()
                            .take(need)
                            .map(|o| o.value.clone())
                            .collect(),
                    )
                }
            }
        }),
        ScriptMessage::PromptForList(info) => Some(match info.defaults() {
            Some(defaults) => ClientMessage::Array(defaults),
            None => ClientMessage::Array(Vec::new()),
        }),
        ScriptMessage::PromptForEditor(info) => Some(match info.default() {
            Some(default) => ClientMessage::String(default),
            None if info.optional => ClientMessage::None,
            None => ClientMessage::String(String::new()),
        }),
        _ => None,
    }
}

impl ScriptIoHandle for ProbeDriver {
    fn send(&self, request: ScriptMessage) -> Result<(), IoError> {
        let mut state = self.state.lock().expect("probe state lock");
        if let Some(envelope) = PromptEnvelope::from_script_message(&request) {
            if state.prompts.len() >= self.prompt_budget {
                state.budget_hit = true;
                state.queued.push_back(ClientMessage::Abort);
                return Ok(());
            }
            let answer = self
                .override_for(envelope.key.as_deref())
                .or_else(|| synthesize(&request))
                .unwrap_or(ClientMessage::None);
            state.prompts.push(envelope);
            state.queued.push_back(answer);
            return Ok(());
        }
        match request {
            ScriptMessage::WriteFile(_) | ScriptMessage::WriteDirectory(_) => {
                // Acknowledged, never written — the probe observes, it
                // does not scaffold.
                state.queued.push_back(ClientMessage::Ack);
            }
            // Logs, prints, completion signals: no reply expected.
            _ => {}
        }
        Ok(())
    }

    fn receive(&self) -> Result<ClientMessage, IoError> {
        let mut state = self.state.lock().expect("probe state lock");
        state
            .queued
            .pop_front()
            .ok_or(IoError::ClientDisconnected)
    }
}
