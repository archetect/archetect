use serde::{Deserialize, Serialize};

use crate::commands::prompt_info::{PromptInfo, PromptInfoItemsRestrictions};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ListPromptInfo {
    pub message: String,
    pub key: Option<String>,
    pub defaults: Option<Vec<String>>,
    pub help: Option<String>,
    pub placeholder: Option<String>,
    pub optional: bool,
    pub min_items: Option<usize>,
    pub max_items: Option<usize>,
}

impl PromptInfo for ListPromptInfo {
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

impl PromptInfoItemsRestrictions for ListPromptInfo {
    fn min_items(&self) -> Option<usize> {
        self.min_items
    }

    fn set_min_items(&mut self, value: Option<usize>) {
        self.min_items = value;
    }

    fn max_items(&self) -> Option<usize> {
        self.max_items
    }

    fn set_max_items(&mut self, value: Option<usize>) {
        self.max_items = value;
    }
}

//noinspection DuplicatedCode
impl ListPromptInfo {
    pub fn new<M: Into<String>, K: AsRef<str>>(message: M, key: Option<K>) -> Self {
        ListPromptInfo {
            message: message.into(),
            key: key.map(|v| v.as_ref().to_string()),
            defaults: Default::default(),
            help: Default::default(),
            placeholder: Default::default(),
            min_items: Default::default(),
            max_items: Default::default(),
            optional: Default::default(),
        }
    }

    pub fn defaults(&self) -> Option<Vec<String>> {
        self.defaults.clone()
    }

    pub fn set_default(&mut self, value: Option<Vec<String>>) {
        self.defaults = value;
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

    pub fn with_defaults(mut self, defaults: Option<Vec<String>>) -> Self {
        self.defaults = defaults;
        self
    }

    pub fn with_min_items(mut self, min_items: Option<usize>) -> Self {
        self.min_items = min_items;
        self
    }

    pub fn with_max_items(mut self, max_items: Option<usize>) -> Self {
        self.max_items = max_items;
        self
    }
}
