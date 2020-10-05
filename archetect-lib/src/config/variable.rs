#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
pub struct VariableInfo {
    #[serde(flatten)]
    value_info: Option<ValueInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    required: Option<bool>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    variable_type: Option<VariableType>,
}

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
pub enum ValueInfo {
    #[serde(rename = "value")]
    Value(String),
    #[serde(rename = "default")]
    Default(String),
}

impl VariableInfo {
    pub fn new() -> VariableInfoBuilder {
        VariableInfoBuilder {
            variable_info: VariableInfo {
                value_info: None,
                prompt: None,
                required: None,
                variable_type: None,
            },
        }
    }

    pub fn with_default<D: Into<String>>(default: D) -> VariableInfoBuilder {
        VariableInfoBuilder {
            variable_info: VariableInfo {
                value_info: Some(ValueInfo::Default(default.into())),
                prompt: None,
                required: None,
                variable_type: None,
            },
        }
    }

    pub fn with_value<V: Into<String>>(value: V) -> VariableInfoBuilder {
        VariableInfoBuilder {
            variable_info: VariableInfo {
                value_info: Some(ValueInfo::Value(value.into())),
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
                value_info: None,
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

    pub fn value_info(&self) -> Option<&ValueInfo> {
        self.value_info.as_ref()
    }

    pub fn value(&self) -> Option<&str> {
        match &self.value_info {
            Some(ValueInfo::Value(value)) => Some(value),
            _ => None
        }
    }

    pub fn default(&self) -> Option<&str> {
        match &self.value_info {
            Some(ValueInfo::Default(default)) => Some(default),
            _ => None
        }
    }

    pub fn variable_type(&self) -> VariableType {
        self.variable_type.clone().unwrap_or(VariableType::String)
    }

    pub fn required(&self) -> bool {
        self.required.unwrap_or(true)
    }

    pub fn has_derived_value(&self) -> bool {
        self.value().is_some()
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
        self.variable_info.value_info = Some(ValueInfo::Value(value.into()));
        self
    }

    pub fn with_default<D: Into<String>>(mut self, default: D) -> VariableInfoBuilder {
        self.variable_info.value_info = Some(ValueInfo::Default(default.into()));
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
