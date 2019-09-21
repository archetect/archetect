

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
pub struct Variable {
    #[serde(alias = "name")]
    #[serde(alias = "identifier")]
    #[serde(rename = "variable")]
    identifier: String,
    #[serde(alias = "default")]
    value: Option<String>,
    default: Option<String>,
    prompt: Option<String>,
    inherit: Option<bool>,
}

impl Variable {
    pub fn new_with_default<I: Into<String>, P: Into<String>, D: Into<String>>(identifier: I, prompt: P, default: D) -> Variable {
        Variable{
            identifier: identifier.into(),
            value: None,
            default: Some(default.into()),
            prompt: Some(prompt.into()),
            inherit: None
        }
    }

    pub fn new_with_value<I: Into<String>, V: Into<String>>(identifier: I, value: V) -> Variable {
        Variable {
            identifier: identifier.into(),
            value: Some(value.into()),
            default: None,
            prompt: None,
            inherit: None
        }
    }

    pub fn new<I: Into<String>>(identifier: I) -> VariableBuilder {
        VariableBuilder {
            variable: Variable {
                prompt: None,
                identifier: identifier.into(),
                value: None,
                default: None,
                inherit: None,
            },
        }
    }

    pub fn with_default<D: Into<String>>(mut self, default: D) -> Variable {
        self.default = Some(default.into());
        self
    }

    pub fn with_value<V: Into<String>>(mut self, value: V) -> Variable {
        self.value = Some(value.into());
        self
    }

    pub fn with_prompt(mut self, value: &str) -> Variable {
        self.prompt = Some(value.into());
        self
    }

    pub fn prompt(&self) -> Option<&str> {
        match &self.prompt {
            Some(prompt) => Some(&prompt),
            None => None,
        }
    }

    pub fn identifier(&self) -> &str {
        &self.identifier
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

    pub fn set_inheritable(&mut self, inheritable: Option<bool>) {
        self.inherit = inheritable
    }

    pub fn with_inheritable(mut self, inheritable: bool) -> Variable {
        self.set_inheritable(Some(inheritable));
        self
    }
}

pub struct VariableBuilder {
    variable: Variable,
}

impl VariableBuilder {
    pub fn with_prompt(mut self, prompt: &str) -> Variable {
        self.variable.prompt = Some(prompt.into());
        self.variable
    }

    pub fn with_value<V: Into<String>>(mut self, value: V) -> Variable {
        self.variable.value = Some(value.into());
        self.variable
    }

    pub fn with_default<D: Into<String>>(mut self, default: D) -> Variable {
        self.variable.default = Some(default.into());
        self.variable
    }
}
