use serde::Serialize;

#[allow(unused_imports)]
pub use archetect_api::{EnvelopeOption, PromptConstraints, PromptEnvelope, PromptType};

use archetect_api::ScriptMessage;

#[derive(Clone, Debug, Serialize)]
pub struct LogEntry {
    pub level: String,
    pub message: String,
}

impl LogEntry {
    pub fn from_script_message(msg: &ScriptMessage) -> Option<Self> {
        match msg {
            ScriptMessage::LogTrace(m) => Some(Self { level: "trace".into(), message: m.clone() }),
            ScriptMessage::LogDebug(m) => Some(Self { level: "debug".into(), message: m.clone() }),
            ScriptMessage::LogInfo(m) => Some(Self { level: "info".into(), message: m.clone() }),
            ScriptMessage::LogWarn(m) => Some(Self { level: "warn".into(), message: m.clone() }),
            ScriptMessage::LogError(m) => Some(Self { level: "error".into(), message: m.clone() }),
            ScriptMessage::Print(m) => Some(Self { level: "print".into(), message: m.clone() }),
            ScriptMessage::Display(m) => Some(Self { level: "display".into(), message: m.clone() }),
            _ => None,
        }
    }
}

// ── Catalog tool response types ────────────────────────────────────

/// A single catalog entry as presented to MCP clients.
/// This is the stable MCP-facing DTO — decoupled from internal `IndexEntry`.
#[derive(Clone, Debug, Serialize)]
pub struct CatalogEntryInfo {
    pub name: String,
    pub path: String,
    pub description: String,
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub languages: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frameworks: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    /// The archetype's declared input contract (prompts + switches),
    /// included only when a single entry is addressed directly — this is
    /// where agents learn a render's switches before starting a session.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interface: Option<serde_json::Value>,
}

impl CatalogEntryInfo {
    pub fn from_index_entry(entry: &archetect_core::catalog::catalog_index::IndexEntry) -> Self {
        let (languages, frameworks, tags) = match &entry.metadata {
            Some(meta) => (
                non_empty_vec(&meta.languages),
                non_empty_vec(&meta.frameworks),
                non_empty_vec(&meta.tags),
            ),
            None => (None, None, None),
        };

        CatalogEntryInfo {
            name: entry.name.clone(),
            path: entry.path.clone(),
            description: entry.description.clone(),
            kind: match entry.kind {
                archetect_core::catalog::catalog_index::IndexEntryKind::Group => "group".to_owned(),
                archetect_core::catalog::catalog_index::IndexEntryKind::Leaf => "leaf".to_owned(),
            },
            source: entry.source.clone(),
            languages,
            frameworks,
            tags,
            interface: None,
        }
    }

    /// Like `from_index_entry`, but carrying the entry's declared
    /// interface. Used when a single entry is addressed directly;
    /// listings omit the interface to keep result sets scannable.
    pub fn from_index_entry_detailed(
        entry: &archetect_core::catalog::catalog_index::IndexEntry,
    ) -> Self {
        let mut info = Self::from_index_entry(entry);
        info.interface = entry
            .interface
            .as_ref()
            .and_then(|iface| serde_json::to_value(iface).ok());
        info
    }
}

fn non_empty_vec(v: &[String]) -> Option<Vec<String>> {
    if v.is_empty() { None } else { Some(v.to_vec()) }
}

#[derive(Clone, Debug, Serialize)]
pub struct CatalogBrowseResponse {
    pub path: String,
    pub entries: Vec<CatalogEntryInfo>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CatalogSearchResponse {
    pub query: String,
    pub results: Vec<CatalogEntryInfo>,
}

// ── Render tool response types ────────────────────────────────────

/// The JSON response returned from render/respond/cancel tool calls.
#[derive(Clone, Debug, Serialize)]
pub struct ToolResponse {
    pub status: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub logs: Vec<LogEntry>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub files_written: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<PromptEnvelope>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl ToolResponse {
    pub fn prompting(logs: Vec<LogEntry>, files_written: Vec<String>, prompt: PromptEnvelope) -> Self {
        Self {
            status: "prompting".into(),
            logs,
            files_written,
            prompt: Some(prompt),
            message: None,
        }
    }

    pub fn complete(logs: Vec<LogEntry>, files_written: Vec<String>) -> Self {
        Self {
            status: "complete".into(),
            logs,
            files_written,
            prompt: None,
            message: None,
        }
    }

    pub fn error(msg: impl Into<String>) -> Self {
        Self {
            status: "error".into(),
            logs: vec![],
            files_written: vec![],
            prompt: None,
            message: Some(msg.into()),
        }
    }

    pub fn cancelled() -> Self {
        Self {
            status: "cancelled".into(),
            logs: vec![],
            files_written: vec![],
            prompt: None,
            message: None,
        }
    }
}
