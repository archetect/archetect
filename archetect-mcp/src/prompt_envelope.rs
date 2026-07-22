use serde::Serialize;

use archetect_api::{
    BoolPromptInfo, EditorPromptInfo, IntPromptInfo, ListPromptInfo,
    MultiSelectPromptInfo, PromptInfo, PromptInfoItemsRestrictions,
    PromptInfoLengthRestrictions, PromptOption, ScriptMessage,
    SelectPromptInfo, TextPromptInfo,
};

/// One selectable choice as the envelope presents it: the VALUE is what
/// `respond`/answers supply and what gets stored; the label is display.
#[derive(Clone, Debug, Serialize)]
pub struct EnvelopeOption {
    pub value: String,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub help: Option<String>,
}

impl EnvelopeOption {
    fn from_options(options: &[PromptOption]) -> Vec<EnvelopeOption> {
        options
            .iter()
            .map(|o| EnvelopeOption {
                value: o.value.clone(),
                label: o.label().to_string(),
                help: o.help.clone(),
            })
            .collect()
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PromptType {
    Text,
    Int,
    Bool,
    List,
    Select,
    MultiSelect,
    Editor,
}

#[derive(Clone, Debug, Serialize)]
pub struct PromptConstraints {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_items: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_items: Option<usize>,
}

#[derive(Clone, Debug, Serialize)]
pub struct PromptEnvelope {
    #[serde(rename = "type")]
    pub prompt_type: PromptType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<EnvelopeOption>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub help: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<String>,
    pub optional: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub constraints: Option<PromptConstraints>,
    /// Regex the value must satisfy (text prompts) — enforced by the
    /// runtime; carried here so clients can validate before responding.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
    /// Author-declared UI section label.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    /// Opaque author-supplied UI metadata, passed through untouched.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ui: Option<serde_json::Value>,
}

impl PromptEnvelope {
    pub fn from_script_message(msg: &ScriptMessage) -> Option<Self> {
        match msg {
            ScriptMessage::PromptForText(info) => Some(Self::from_text(info)),
            ScriptMessage::PromptForInt(info) => Some(Self::from_int(info)),
            ScriptMessage::PromptForBool(info) => Some(Self::from_bool(info)),
            ScriptMessage::PromptForList(info) => Some(Self::from_list(info)),
            ScriptMessage::PromptForSelect(info) => Some(Self::from_select(info)),
            ScriptMessage::PromptForMultiSelect(info) => Some(Self::from_multiselect(info)),
            ScriptMessage::PromptForEditor(info) => Some(Self::from_editor(info)),
            _ => None,
        }
    }

    fn from_text(info: &TextPromptInfo) -> Self {
        Self {
            prompt_type: PromptType::Text,
            key: info.key().map(String::from),
            message: info.message().to_string(),
            default: info.default().map(|s| serde_json::Value::String(s)),
            options: None,
            help: info.help().map(String::from),
            placeholder: info.placeholder().map(String::from),
            optional: info.optional(),
            constraints: Some(PromptConstraints {
                min: info.min(),
                max: info.max(),
                min_items: None,
                max_items: None,
            }),
            pattern: info.pattern.clone(),
            group: info.group.clone(),
            ui: info.ui.clone(),
        }
    }

    fn from_int(info: &IntPromptInfo) -> Self {
        Self {
            prompt_type: PromptType::Int,
            key: info.key().map(String::from),
            message: info.message().to_string(),
            default: info.default().map(|i| serde_json::Value::Number(i.into())),
            options: None,
            help: info.help().map(String::from),
            placeholder: info.placeholder().map(String::from),
            optional: info.optional(),
            constraints: Some(PromptConstraints {
                min: info.min(),
                max: info.max(),
                min_items: None,
                max_items: None,
            }),
            pattern: None,
            group: info.group.clone(),
            ui: info.ui.clone(),
        }
    }

    fn from_bool(info: &BoolPromptInfo) -> Self {
        Self {
            prompt_type: PromptType::Bool,
            key: info.key().map(String::from),
            message: info.message().to_string(),
            default: info.default().map(serde_json::Value::Bool),
            options: None,
            help: info.help().map(String::from),
            placeholder: info.placeholder().map(String::from),
            optional: info.optional(),
            constraints: None,
            pattern: None,
            group: info.group.clone(),
            ui: info.ui.clone(),
        }
    }

    fn from_list(info: &ListPromptInfo) -> Self {
        Self {
            prompt_type: PromptType::List,
            key: info.key().map(String::from),
            message: info.message().to_string(),
            default: info.defaults().map(|d| serde_json::Value::Array(
                d.iter().map(|s| serde_json::Value::String(s.clone())).collect()
            )),
            options: None,
            help: info.help().map(String::from),
            placeholder: info.placeholder().map(String::from),
            optional: info.optional(),
            constraints: Some(PromptConstraints {
                min: None,
                max: None,
                min_items: info.min_items(),
                max_items: info.max_items(),
            }),
            pattern: None,
            group: info.group.clone(),
            ui: info.ui.clone(),
        }
    }

    fn from_select(info: &SelectPromptInfo) -> Self {
        Self {
            prompt_type: PromptType::Select,
            key: info.key().map(String::from),
            message: info.message().to_string(),
            default: info.default().map(serde_json::Value::String),
            options: Some(EnvelopeOption::from_options(info.options())),
            help: info.help().map(String::from),
            placeholder: info.placeholder().map(String::from),
            optional: info.optional(),
            constraints: None,
            pattern: None,
            group: info.group.clone(),
            ui: info.ui.clone(),
        }
    }

    fn from_multiselect(info: &MultiSelectPromptInfo) -> Self {
        Self {
            prompt_type: PromptType::MultiSelect,
            key: info.key().map(String::from),
            message: info.message().to_string(),
            default: info.defaults().map(|d| serde_json::Value::Array(
                d.iter().map(|s| serde_json::Value::String(s.clone())).collect()
            )),
            options: Some(EnvelopeOption::from_options(info.options())),
            help: info.help().map(String::from),
            placeholder: info.placeholder().map(String::from),
            optional: info.optional(),
            constraints: Some(PromptConstraints {
                min: None,
                max: None,
                min_items: info.min_items(),
                max_items: info.max_items(),
            }),
            pattern: None,
            group: info.group.clone(),
            ui: info.ui.clone(),
        }
    }

    fn from_editor(info: &EditorPromptInfo) -> Self {
        Self {
            prompt_type: PromptType::Editor,
            key: info.key().map(String::from),
            message: info.message().to_string(),
            default: info.default().map(|s| serde_json::Value::String(s)),
            options: None,
            help: info.help().map(String::from),
            placeholder: info.placeholder().map(String::from),
            optional: info.optional(),
            constraints: Some(PromptConstraints {
                min: info.min(),
                max: info.max(),
                min_items: None,
                max_items: None,
            }),
            pattern: None,
            group: info.group.clone(),
            ui: info.ui.clone(),
        }
    }
}

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
