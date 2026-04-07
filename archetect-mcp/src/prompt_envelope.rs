use serde::Serialize;

use archetect_api::{
    BoolPromptInfo, EditorPromptInfo, IntPromptInfo, ListPromptInfo,
    MultiSelectPromptInfo, PromptInfo, PromptInfoItemsRestrictions,
    PromptInfoLengthRestrictions, ScriptMessage,
    SelectPromptInfo, TextPromptInfo,
};

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
    pub options: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub help: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<String>,
    pub optional: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub constraints: Option<PromptConstraints>,
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
        }
    }

    fn from_select(info: &SelectPromptInfo) -> Self {
        Self {
            prompt_type: PromptType::Select,
            key: info.key().map(String::from),
            message: info.message().to_string(),
            default: info.default().map(serde_json::Value::String),
            options: Some(info.options().to_vec()),
            help: info.help().map(String::from),
            placeholder: info.placeholder().map(String::from),
            optional: info.optional(),
            constraints: None,
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
            options: Some(info.options().to_vec()),
            help: info.help().map(String::from),
            placeholder: info.placeholder().map(String::from),
            optional: info.optional(),
            constraints: Some(PromptConstraints {
                min: None,
                max: None,
                min_items: info.min_items(),
                max_items: info.max_items(),
            }),
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
