#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
pub struct VariableInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    default: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    required: Option<bool>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    variable_type: Option<VariableType>,
}

impl VariableInfo {
    pub fn builder() -> VariableInfoBuilder {
        VariableInfoBuilder {
            variable_info: VariableInfo {
                value: None,
                default: None,
                prompt: None,
                required: None,
                variable_type: None,
            },
        }
    }

    pub fn with_default<D: Into<String>>(default: D) -> VariableInfoBuilder {
        VariableInfoBuilder {
            variable_info: VariableInfo {
                value: None,
                default: Some(default.into()),
                prompt: None,
                required: None,
                variable_type: None,
            },
        }
    }

    pub fn with_value<V: Into<String>>(value: V) -> VariableInfoBuilder {
        VariableInfoBuilder {
            variable_info: VariableInfo {
                value: Some(value.into()),
                default: None,
                prompt: None,
                required: None,
                variable_type: None,
            },
        }
    }

    pub fn with_prompt<P: Into<String>>(prompt: P) -> VariableInfoBuilder {
        VariableInfoBuilder {
            variable_info: VariableInfo {
                prompt: Some(prompt.into()),
                value: None,
                default: None,
                required: None,
                variable_type: None,
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

    pub fn variable_type(&self) -> VariableType {
        self.variable_type.clone().unwrap_or(VariableType::String)
    }

    pub fn required(&self) -> bool {
        self.required.unwrap_or(true)
    }

    pub fn has_derived_value(&self) -> bool {
        self.value.is_some()
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
pub enum VariableType {
    #[serde(rename = "string")]
    String,
    #[serde(rename = "int")]
    Int,
    #[serde(rename = "bool")]
    Bool,
    #[serde(rename = "enum")]
    Enum(Vec<String>),
    #[serde(rename = "array", alias = "list")]
    Array,
}

pub struct VariableInfoBuilder {
    variable_info: VariableInfo,
}

impl VariableInfoBuilder {
    pub fn with_prompt<P: Into<String>>(mut self, prompt: P) -> VariableInfoBuilder {
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

    pub fn with_type(mut self, variable_type: VariableType) -> VariableInfoBuilder {
        self.variable_info.variable_type = Some(variable_type);
        self
    }

    pub fn build(self) -> VariableInfo {
        self.variable_info
    }
}
