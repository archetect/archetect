use crate::commands::prompt_info::PromptInfo;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BoolPromptInfo {
    message: String,
    default: Option<bool>,
    help: Option<String>,
    placeholder: Option<String>,
    optional: bool,
}

impl PromptInfo for BoolPromptInfo {
    fn message(&self) -> &str {
        return self.message.as_ref();
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
impl BoolPromptInfo {
    pub fn new<M: Into<String>>(message: M) -> Self {
        BoolPromptInfo {
            message: message.into(),
            default: Default::default(),
            help: Default::default(),
            placeholder: Default::default(),
            optional: Default::default(),
        }
    }

    pub fn message(&self) -> &str {
        self.message.as_str()
    }

    pub fn default(&self) -> Option<bool> {
        self.default
    }

    pub fn help(&self) -> Option<&str> {
        self.help.as_deref()
    }

    pub fn placeholder(&self) -> Option<&str> {
        self.placeholder.as_deref()
    }

    pub fn with_default(mut self, value: Option<bool>) -> Self {
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
}
