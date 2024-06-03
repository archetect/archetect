use std::ops::Deref;

use serde::{Deserialize, Serialize};

use crate::commands::prompt_info::PromptInfo;
use crate::PromptInfoPageable;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SelectPromptInfo {
    message: String,
    key: Option<String>,
    options: Vec<String>,
    default: Option<String>,
    help: Option<String>,
    placeholder: Option<String>,
    optional: bool,
    page_size: Option<usize>,
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

impl PromptInfoPageable for SelectPromptInfo {
    fn page_size(&self) -> Option<usize> {
        self.page_size
    }

    fn set_page_size(&mut self, value: Option<usize>) {
        self.page_size = value;
    }
}

//noinspection DuplicatedCode
impl SelectPromptInfo {
    pub fn new<M: Into<String>, K: AsRef<str>>(message: M, key: Option<K>, options: Vec<String>) -> Self {
        SelectPromptInfo {
            message: message.into(),
            key: key.map(|v| v.as_ref().to_string()),
            options,
            default: Default::default(),
            help: Default::default(),
            placeholder: Default::default(),
            optional: Default::default(),
            page_size: Some(10),
        }
    }

    pub fn options(&self) -> &[String] {
        self.options.deref()
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
}
