use crate::commands::prompt_info::PromptInfo;
use serde::{Deserialize, Serialize};
use std::ops::Deref;
use crate::PromptInfoPageable;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SelectPromptInfo {
    pub message: String,
    pub key: Option<String>,
    pub options: Vec<String>,
    pub default: Option<String>,
    pub help: Option<String>,
    pub placeholder: Option<String>,
    pub optional: bool,
    pub page_size: Option<usize>,
    /// When true, the client should append an "other" entry to the menu;
    /// selecting it triggers a free-text prompt that returns whatever the
    /// user types. Lets a `prompt_select` accept values outside the curated
    /// list without forcing the author to fall back to `prompt_text`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub allow_other: bool,
    /// Label for the "other" menu entry. Defaults to "Other..." when None.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub other_label: Option<String>,
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
            key: key.map(|v|v.as_ref().to_string()),
            options,
            default: Default::default(),
            help: Default::default(),
            placeholder: Default::default(),
            optional: Default::default(),
            page_size: Some(10),
            allow_other: false,
            other_label: None,
        }
    }

    pub fn allow_other(&self) -> bool {
        self.allow_other
    }

    pub fn other_label(&self) -> &str {
        self.other_label.as_deref().unwrap_or("Other...")
    }

    pub fn with_allow_other(mut self, allow: bool) -> Self {
        self.allow_other = allow;
        self
    }

    pub fn with_other_label(mut self, label: Option<String>) -> Self {
        self.other_label = label;
        self
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
