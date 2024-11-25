use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};
use archetect_templating::UndefinedBehavior as MinijinjaUndefinedBehavior;

const DEFAULT_CONTENT_DIRECTORY: &str = ".";
const DEFAULT_TEMPLATES_DIRECTORY: &str = "templates";

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TemplatingConfig {
    #[serde(default = "default_content_directory")]
    content: Utf8PathBuf,
    #[serde(default = "default_templates_directory")]
    templates: Utf8PathBuf,
    #[serde(default = "default_undefined_behavior")]
    undefined_behavior: UndefinedBehavior,
}

impl TemplatingConfig {
    pub fn content_directory(&self) -> &Utf8Path {
        &self.content
    }
    pub fn templates_directory(&self) -> &Utf8Path {
        &self.templates
    }

    pub fn undefined_behavior(&self) -> UndefinedBehavior {
        self.undefined_behavior
    }
}

impl Default for TemplatingConfig {
    fn default() -> Self {
        TemplatingConfig {
            content: default_content_directory(),
            templates: default_templates_directory(),
            undefined_behavior: default_undefined_behavior(),
        }
    }
}

fn default_content_directory() -> Utf8PathBuf {
    Utf8PathBuf::from(DEFAULT_CONTENT_DIRECTORY)
}

fn default_templates_directory() -> Utf8PathBuf {
    Utf8PathBuf::from(DEFAULT_TEMPLATES_DIRECTORY)
}

fn default_undefined_behavior() -> UndefinedBehavior {
    UndefinedBehavior::Strict
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UndefinedBehavior {
    Lenient,
    Chainable,
    Strict,
}

impl UndefinedBehavior {
    pub fn to_minijinja(&self) -> MinijinjaUndefinedBehavior {
        match self {
            UndefinedBehavior::Lenient => MinijinjaUndefinedBehavior::Lenient,
            UndefinedBehavior::Chainable => MinijinjaUndefinedBehavior::Chainable,
            UndefinedBehavior::Strict => MinijinjaUndefinedBehavior::Strict,
        }
    }
}
