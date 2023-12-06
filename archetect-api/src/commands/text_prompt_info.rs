use crate::commands::prompt_info::{PromptInfo, PromptInfoLengthRestrictions};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TextPromptInfo {
    message: String,
    key: Option<String>,
    default: Option<String>,
    min: Option<i64>,
    max: Option<i64>,
    help: Option<String>,
    placeholder: Option<String>,
    optional: bool,
}

impl PromptInfo for TextPromptInfo {
    fn message(&self) -> &str {
        self.message.as_ref()
    }

    fn optional(&self) -> bool {
        self.optional
    }

    fn help(&self) -> Option<&str> {
        self.help.as_deref()
    }

    fn placeholder(&self) -> Option<&str> {
        self.placeholder.as_deref()
    }

    fn set_optional(&mut self, value: bool) {
        self.optional = value;
    }

    fn set_help(&mut self, value: Option<String>) {
        self.help = value;
    }

    fn set_placeholder(&mut self, value: Option<String>) {
        self.placeholder = value;
    }

    fn key(&self) -> Option<&str> {
        self.key.as_deref()
    }
}

impl PromptInfoLengthRestrictions for TextPromptInfo {
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
impl TextPromptInfo {
    pub fn new<M: Into<String>, K: AsRef<str>>(message: M, key: Option<K>) -> Self {
        TextPromptInfo {
            message: message.into(),
            key: key.map(|v|v.as_ref().to_string()),
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
}
