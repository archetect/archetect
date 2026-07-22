//! The prompt envelope — one prompt's full declaration, as clients see it.
//!
//! Shared by every consumer of the prompt stream: the MCP session, the
//! interface probe, and (eventually) the gRPC describe surface. One
//! envelope type means one JSON shape everywhere.

use serde::{Deserialize, Serialize};

use crate::commands::{
    BoolPromptInfo, EditorPromptInfo, IntPromptInfo, ListPromptInfo,
    MultiSelectPromptInfo, PromptInfo, PromptInfoItemsRestrictions,
    PromptInfoLengthRestrictions, PromptOption, ScriptMessage,
    SelectPromptInfo, TextPromptInfo,
};

/// One selectable choice as the envelope presents it: the VALUE is what
/// `respond`/answers supply and what gets stored; the label is display.
#[derive(Clone, Debug, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Serialize, Deserialize)]
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

