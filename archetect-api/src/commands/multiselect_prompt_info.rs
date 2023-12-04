use crate::commands::prompt_info::PromptInfo;
use serde::{Deserialize, Serialize};
use std::ops::Deref;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MultiSelectPromptInfo {
    message: String,
    options: Vec<String>,
    defaults: Option<Vec<String>>,
    help: Option<String>,
    placeholder: Option<String>,
    optional: bool,
}

impl PromptInfo for MultiSelectPromptInfo {
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
impl MultiSelectPromptInfo {
    pub fn new<M: Into<String>>(message: M, options: Vec<String>) -> Self {
        MultiSelectPromptInfo {
            message: message.into(),
            options,
            defaults: Default::default(),
            help: Default::default(),
            placeholder: Default::default(),
            optional: Default::default(),
        }
    }

    pub fn options(&self) -> &[String] {
        self.options.deref()
    }

    pub fn defaults(&self) -> Option<&[String]> {
        self.defaults.as_deref()
    }

    pub fn with_defaults(mut self, defaults: Option<Vec<String>>) -> Self {
        self.defaults = defaults;
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
