use crate::commands::prompt_info::{PromptInfo, PromptInfoPageable};
use crate::commands::prompt_option::PromptOption;
use serde::{Deserialize, Serialize};
use std::ops::Deref;
use crate::PromptInfoItemsRestrictions;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MultiSelectPromptInfo {
    pub message: String,
    pub key: Option<String>,
    pub options: Vec<PromptOption>,
    pub defaults: Option<Vec<String>>,
    pub help: Option<String>,
    pub placeholder: Option<String>,
    pub optional: bool,
    pub min_items: Option<usize>,
    pub max_items: Option<usize>,
    pub page_size: Option<usize>,
    /// Optional UI section label — metadata carried to clients untouched.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    /// Opaque author-supplied UI metadata, passed through to clients untouched.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ui: Option<serde_json::Value>,
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
    pub fn new<M: Into<String>, K: AsRef<str>, O: Into<PromptOption>>(
        message: M,
        key: Option<K>,
        options: Vec<O>,
    ) -> Self {
        MultiSelectPromptInfo {
            message: message.into(),
            key: key.map(|v|v.as_ref().to_string()),
            options: options.into_iter().map(Into::into).collect(),
            defaults: Default::default(),
            help: Default::default(),
            placeholder: Default::default(),
            optional: Default::default(),
            min_items: Default::default(),
            max_items: Default::default(),
            page_size: Some(10),
            group: None,
            ui: None,
        }
    }

    pub fn options(&self) -> &[PromptOption] {
        self.options.deref()
    }

    /// The answer domain: each option's stored value.
    pub fn option_values(&self) -> Vec<String> {
        self.options.iter().map(|o| o.value.clone()).collect()
    }

    pub fn defaults(&self) -> Option<Vec<String>> {
        self.defaults.clone()
    }

    pub fn set_defaults(&mut self, defaults: Option<Vec<String>>) {
        self.defaults = defaults;
    }
}
