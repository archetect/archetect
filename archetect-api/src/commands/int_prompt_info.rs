use serde::{Deserialize, Serialize};
use crate::commands::prompt_info::PromptInfo;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct IntPromptInfo {
    message: String,
    default: Option<i64>,
    min: Option<i64>,
    max: Option<i64>,
    help: Option<String>,
    placeholder: Option<String>,
    optional: bool,
}

impl PromptInfo for IntPromptInfo {
    fn message(&self) -> &str {
        return self.message.as_ref()
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
impl IntPromptInfo {
    pub fn new<M: Into<String>>(message: M) -> Self {
        IntPromptInfo {
            message: message.into(),
            default: Default::default(),
            min: Default::default(),
            max: Default::default(),
            help: Default::default(),
            placeholder: Default::default(),
            optional: Default::default(),
        }
    }

    pub fn default(&self) -> Option<i64> {
        self.default.clone()
    }

    pub fn with_default(mut self, value: Option<i64>) -> Self {
        self.default = value;
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
    pub fn min(&self) -> Option<i64> {
        self.min
    }

    pub fn max(&self) -> Option<i64> {
        self.max
    }

    pub fn with_min(mut self, min: Option<i64>) -> Self {
        self.min = min;
        self
    }
    pub fn with_max(mut self, max: Option<i64>) -> Self {
        self.max = max;
        self
    }

    pub fn with_optional(mut self, optional: bool) -> Self {
        self.optional = optional;
        self
    }
}
