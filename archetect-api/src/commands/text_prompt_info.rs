use serde::{Deserialize, Serialize};
use crate::commands::prompt_info::PromptInfo;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TextPromptInfo {
    message: String,
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
}

//noinspection DuplicatedCode
impl TextPromptInfo {
    pub fn new<M: Into<String>>(message: M) -> Self {
        TextPromptInfo {
            message: message.into(),
            default: Default::default(),
            min: Some(1),
            max: Default::default(),
            help: Default::default(),
            placeholder: Default::default(),
            optional: Default::default(),
        }
    }
    pub fn default(&self) -> Option<&str> {
        self.default.as_deref()
    }

    pub fn with_default<T: Into<String>>(mut self, value: Option<T>) -> Self {
        self.default = value.map(|v| v.into());
        self
    }

    pub fn with_help<T: Into<String>>(mut self, value: Option<T>) -> Self {
        self.help = value.map(|v| v.into());
        self
    }

    pub fn with_placeholder<T: Into<String>>(mut self, value: Option<T>) -> Self {
        self.placeholder = value.map(|v| v.into());
        self
    }
    pub fn min(&self) -> Option<i64> {
        self.min
    }
    pub fn max(&self) -> Option<i64> {
        self.max
    }
}
