use camino::{Utf8Path, Utf8PathBuf};

const DEFAULT_CONTENT_DIRECTORY: &str = ".";
const DEFAULT_TEMPLATES_DIRECTORY: &str = "templates";

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TemplatingConfig {
    #[serde(default = "default_content_directory")]
    content: Utf8PathBuf,
    #[serde(default = "default_templates_directory")]
    templates: Utf8PathBuf,
}

impl TemplatingConfig {
    pub fn content_directory(&self) -> &Utf8Path {
        &self.content
    }
    pub fn templates_directory(&self) -> &Utf8Path {
        &self.templates
    }
}

impl Default for TemplatingConfig {
    fn default() -> Self {
        TemplatingConfig {
            content: default_content_directory(),
            templates: default_templates_directory(),
        }
    }
}

fn default_content_directory() -> Utf8PathBuf {
    Utf8PathBuf::from(DEFAULT_CONTENT_DIRECTORY)
}

fn default_templates_directory() -> Utf8PathBuf {
    Utf8PathBuf::from(DEFAULT_TEMPLATES_DIRECTORY)
}
