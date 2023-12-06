use crate::commands::prompt_info::{PromptInfo, PromptInfoItemsRestrictions};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ListPromptInfo {
    message: String,
    key: Option<String>,
    defaults: Option<Vec<String>>,
    help: Option<String>,
    placeholder: Option<String>,
    optional: bool,
    min_items: Option<usize>,
    max_items: Option<usize>,
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
        self.max_items = value;
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
}
