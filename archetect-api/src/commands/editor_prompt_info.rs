use serde::{Deserialize, Serialize};

use crate::commands::prompt_info::PromptInfo;
use crate::PromptInfoLengthRestrictions;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EditorPromptInfo {
    pub message: String,
    pub key: Option<String>,
    pub default: Option<String>,
    pub min: Option<i64>,
    pub max: Option<i64>,
    pub help: Option<String>,
    pub placeholder: Option<String>,
    pub optional: bool,
}

impl PromptInfo for EditorPromptInfo {
    fn message(&self) -> &str {
        self.message.as_ref()
    }

    fn key(&self) -> Option<&str> {
        self.key.as_deref()
    }

    fn optional(&self) -> bool {
        self.optional
    }

    fn set_optional(&mut self, value: bool) {
        self.optional = value;
    }

    fn help(&self) -> Option<&str> {
        self.help.as_deref()
    }

    fn set_help(&mut self, value: Option<String>) {
        self.help = value;
    }

    fn placeholder(&self) -> Option<&str> {
        self.placeholder.as_deref()
    }

    fn set_placeholder(&mut self, value: Option<String>) {
        self.placeholder = value;
    }
}

impl PromptInfoLengthRestrictions for EditorPromptInfo {
    fn min(&self) -> Option<i64> {
        self.min
    }

    fn set_min(&mut self, value: Option<i64>) {
        self.min = value;
    }

    fn max(&self) -> Option<i64> {
        self.max
    }

    fn set_max(&mut self, value: Option<i64>) {
        self.max = value;
    }
}

//noinspection DuplicatedCode
impl EditorPromptInfo {
    pub fn new<M: Into<String>, K: AsRef<str>>(message: M, key: Option<K>) -> Self {
        EditorPromptInfo {
            message: message.into(),
            key: key.map(|v| v.as_ref().to_string()),
            default: Default::default(),
            min: Some(1),
            max: Default::default(),
            help: Default::default(),
            placeholder: Default::default(),
            optional: Default::default(),
        }
    }
    pub fn default(&self) -> Option<String> {
        self.default.clone()
    }

    pub fn with_default(mut self, value: Option<String>) -> Self {
        self.default = value;
        self
    }

    pub fn with_help(mut self, value: Option<String>) -> Self {
        self.help = value;
        self
    }

    pub fn with_placeholder(mut self, value: Option<String>) -> Self {
        self.placeholder = value;
        self
    }

    pub fn with_optional(mut self, optional: bool) -> Self {
        self.optional = optional;
        self
    }

    pub fn with_min(mut self, min: Option<usize>) -> Self {
        // TODO: consolidate on integer type
        self.min = min.map(|v| v as i64);
        self
    }

    pub fn with_max(mut self, max: Option<usize>) -> Self {
        // TODO: consolidate on integer type
        self.max = max.map(|v| v as i64);
        self
    }
}
