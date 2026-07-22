use serde::{Deserialize, Serialize};

/// One choice in a select/multiselect prompt.
///
/// Authors may write options as bare strings (value and label identical)
/// or as rich tables `{ value, label?, help? }` — mirroring the two YAML
/// forms the old declarative interface accepted, but at the prompt call,
/// where the declaration is also the behavior. The VALUE is the contract:
/// it is what answers supply, what defaults name, and what is stored.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct PromptOption {
    /// The value stored (and answered) when this option is chosen.
    pub value: String,
    /// Display label. `None` means "show the value".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Optional per-option help text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub help: Option<String>,
}

impl PromptOption {
    pub fn new<V: Into<String>>(value: V) -> Self {
        PromptOption {
            value: value.into(),
            label: None,
            help: None,
        }
    }

    /// What a UI shows for this option.
    pub fn label(&self) -> &str {
        self.label.as_deref().unwrap_or(&self.value)
    }
}

impl From<String> for PromptOption {
    fn from(value: String) -> Self {
        PromptOption::new(value)
    }
}

impl From<&str> for PromptOption {
    fn from(value: &str) -> Self {
        PromptOption::new(value)
    }
}

impl<'de> Deserialize<'de> for PromptOption {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Raw {
            Short(String),
            Long {
                value: String,
                #[serde(default)]
                label: Option<String>,
                #[serde(default)]
                help: Option<String>,
            },
        }

        match Raw::deserialize(deserializer)? {
            Raw::Short(value) => Ok(PromptOption::new(value)),
            Raw::Long { value, label, help } => Ok(PromptOption { value, label, help }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bare_string_deserializes_to_value_only() {
        let opt: PromptOption = serde_json::from_str(r#""postgres""#).unwrap();
        assert_eq!(opt.value, "postgres");
        assert_eq!(opt.label(), "postgres");
    }

    #[test]
    fn rich_form_round_trips() {
        let opt: PromptOption =
            serde_json::from_str(r#"{"value":"pg","label":"PostgreSQL","help":"Prod"}"#).unwrap();
        assert_eq!(opt.value, "pg");
        assert_eq!(opt.label(), "PostgreSQL");
        assert_eq!(opt.help.as_deref(), Some("Prod"));
    }
}
