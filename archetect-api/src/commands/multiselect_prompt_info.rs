use std::ops::Deref;
use serde::{Deserialize, Serialize};
use crate::commands::prompt_info::PromptInfo;

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

    pub fn with_help<T: Into<String>>(mut self, value: Option<T>) -> Self {
        self.help = value.map(|v| v.into());
        self
    }

    pub fn with_placeholder<T: Into<String>>(mut self, value: Option<T>) -> Self {
        self.placeholder = value.map(|v| v.into());
        self
    }

    pub fn with_optional(mut self, optional: bool) -> Self {
        self.optional = optional;
        self
    }
}
