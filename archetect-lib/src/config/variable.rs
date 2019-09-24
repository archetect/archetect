

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
pub struct VariableInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    default: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    inherit: Option<bool>,
}

impl VariableInfo {
    pub fn with_default<D: Into<String>>(default: D) -> VariableInfoBuilder {
        VariableInfoBuilder {
            variable_info: VariableInfo {
                value: None,
                default: Some(default.into()),
                prompt: None,
                inherit: None,
            }
        }
    }

    pub fn with_value<V: Into<String>>(value: V) -> VariableInfoBuilder {
        VariableInfoBuilder {
            variable_info: VariableInfo {
                value: Some(value.into()),
                default: None,
                prompt: None,
                inherit: None,
            }
        }
    }

    pub fn with_prompt<P: Into<String>>(prompt: P) -> VariableInfoBuilder {
        VariableInfoBuilder {
            variable_info: VariableInfo {
                prompt: Some(prompt.into()),
                value: None,
                default: None,
                inherit: None,
            },
        }
    }

    pub fn prompt(&self) -> Option<&str> {
        match &self.prompt {
            Some(prompt) => Some(&prompt),
            None => None,
        }
    }

    pub fn value(&self) -> Option<&str> {
        match &self.value {
            Some(value) => Some(value.as_str()),
            None => None,
        }
    }

    pub fn default(&self) -> Option<&str> {
        match &self.default {
            Some(default) => Some(default.as_str()),
            None => None,
        }
    }

    pub fn is_inheritable(&self) -> bool {
        self.inherit.unwrap_or(false)
    }
}

pub struct VariableInfoBuilder {
    variable_info: VariableInfo,
}

impl VariableInfoBuilder {
    pub fn with_prompt(mut self, prompt: &str) -> VariableInfoBuilder {
        self.variable_info.prompt = Some(prompt.into());
        self
    }

    pub fn with_value<V: Into<String>>(mut self, value: V) -> VariableInfoBuilder {
        self.variable_info.value = Some(value.into());
        self
    }

    pub fn with_default<D: Into<String>>(mut self, default: D) -> VariableInfoBuilder {
        self.variable_info.default = Some(default.into());
        self
    }

    pub fn inheritable(mut self) -> VariableInfoBuilder {
        self.variable_info.inherit = Some(true);
        self
    }

    pub fn build(self) -> VariableInfo {
        self.variable_info
    }
}
