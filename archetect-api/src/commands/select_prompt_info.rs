use crate::commands::prompt_info::PromptInfo;
use serde::{Deserialize, Serialize};
use std::ops::Deref;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SelectPromptInfo {
    message: String,
    options: Vec<String>,
    default: Option<String>,
    help: Option<String>,
    placeholder: Option<String>,
    optional: bool,
}

impl PromptInfo for SelectPromptInfo {
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
impl SelectPromptInfo {
    pub fn new<M: Into<String>>(message: M, options: Vec<String>) -> Self {
        SelectPromptInfo {
            message: message.into(),
            options,
            default: Default::default(),
            help: Default::default(),
            placeholder: Default::default(),
            optional: Default::default(),
        }
    }

    pub fn options(&self) -> &[String] {
        self.options.deref()
    }

    pub fn default(&self) -> Option<&str> {
        self.default.as_deref()
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
}
