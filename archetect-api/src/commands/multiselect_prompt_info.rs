use std::ops::Deref;

use serde::{Deserialize, Serialize};

use crate::commands::prompt_info::{PromptInfo, PromptInfoPageable};
use crate::PromptInfoItemsRestrictions;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MultiSelectPromptInfo {
    pub message: String,
    pub key: Option<String>,
    pub options: Vec<String>,
    pub defaults: Option<Vec<String>>,
    pub help: Option<String>,
    pub placeholder: Option<String>,
    pub optional: bool,
    pub min_items: Option<usize>,
    pub max_items: Option<usize>,
    pub page_size: Option<usize>,
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

impl PromptInfoItemsRestrictions for MultiSelectPromptInfo {
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

impl PromptInfoPageable for MultiSelectPromptInfo {
    fn page_size(&self) -> Option<usize> {
        self.page_size
    }

    fn set_page_size(&mut self, value: Option<usize>) {
        self.page_size = value;
    }
}

//noinspection DuplicatedCode
impl MultiSelectPromptInfo {
    pub fn new<M: Into<String>, K: AsRef<str>>(message: M, key: Option<K>, options: Vec<String>) -> Self {
        MultiSelectPromptInfo {
            message: message.into(),
            key: key.map(|v| v.as_ref().to_string()),
            options,
            defaults: Default::default(),
            help: Default::default(),
            placeholder: Default::default(),
            optional: Default::default(),
            min_items: Default::default(),
            max_items: Default::default(),
            page_size: Some(10),
        }
    }

    pub fn options(&self) -> &[String] {
        self.options.deref()
    }

    pub fn defaults(&self) -> Option<Vec<String>> {
        self.defaults.clone()
    }

    pub fn set_defaults(&mut self, defaults: Option<Vec<String>>) {
        self.defaults = defaults;
    }
    pub fn with_defaults(mut self, defaults: Option<Vec<String>>) -> Self {
        self.set_defaults(defaults);
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

    pub fn with_min_items(mut self, min_items: Option<usize>) -> Self {
        self.min_items = min_items;
        self
    }

    pub fn with_max_items(mut self, max_items: Option<usize>) -> Self {
        self.max_items = max_items;
        self
    }

    pub fn with_page_size(mut self, page_size: Option<usize>) -> Self {
        self.page_size = page_size;
        self
    }
}
