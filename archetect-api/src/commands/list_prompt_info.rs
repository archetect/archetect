use crate::commands::prompt_info::PromptInfo;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ListPromptInfo {
    message: String,
    defaults: Option<Vec<String>>,
    help: Option<String>,
    placeholder: Option<String>,
    min_items: Option<usize>,
    max_items: Option<usize>,
    optional: bool,
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
}

//noinspection DuplicatedCode
impl ListPromptInfo {
    pub fn new<M: Into<String>>(message: M) -> Self {
        ListPromptInfo {
            message: message.into(),
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

    pub fn with_defaults(mut self, value: Option<Vec<String>>) -> Self {
        self.defaults = value;
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

    pub fn with_min_items(mut self, min_items: Option<usize>) -> Self {
        self.min_items = min_items;
        self
    }

    pub fn with_max_items(mut self, max_items: Option<usize>) -> Self {
        self.max_items = max_items;
        self
    }

    pub fn min_items(&self) -> Option<usize> {
        self.min_items
    }

    pub fn max_items(&self) -> Option<usize> {
        self.max_items
    }

    pub fn with_optional(mut self, optional: bool) -> Self {
        self.optional = optional;
        self
    }
}
