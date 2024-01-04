use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ConfigurationLocalsSection {
    #[serde(skip_serializing_if = "Option::is_none")]
    enabled: Option<bool>,
    paths: Vec<Utf8PathBuf>,
}

impl ConfigurationLocalsSection {
    pub fn enabled(&self) -> bool {
        self.enabled.unwrap_or_default()
    }

    pub fn paths(&self) -> &[Utf8PathBuf] {
        self.paths.as_slice()
    }
}

impl Default for ConfigurationLocalsSection {
    fn default() -> Self {
        let mut paths = vec![];
        paths.push(Utf8PathBuf::from("~/projects/archetypes/"));
        Self {
            enabled: Default::default(),
            paths,
        }
    }
}
