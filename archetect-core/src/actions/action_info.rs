use std::collections::HashSet;

use rhai::Map;

use crate::catalog::CatalogEntry;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RenderGroupInfo {
    pub(crate) entries: Vec<CatalogEntry>,
}

impl RenderGroupInfo {
    pub fn new(entries: Vec<CatalogEntry>) -> RenderGroupInfo {
        RenderGroupInfo {
            entries,
        }
    }

    pub fn entries(&self) -> &Vec<CatalogEntry> {
        &self.entries
    }

    pub fn entries_owned(self) -> Vec<CatalogEntry> {
        self.entries
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RenderCatalogInfo {
    source: String,
}

impl RenderCatalogInfo {
    pub fn new<S: Into<String>>(source: S) -> RenderCatalogInfo {
        RenderCatalogInfo {
            source: source.into(),
        }
    }

    pub fn source(&self) -> &str {
        self.source.as_str()
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RenderArchetypeInfo {
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub answers: Option<Map>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub switches: Option<HashSet<String>>,
    #[serde(rename = "use_defaults",skip_serializing_if = "Option::is_none")]
    pub use_defaults: Option<HashSet<String>>,
    #[serde(rename = "use_defaults_all", skip_serializing_if = "Option::is_none")]
    pub use_defaults_all: Option<bool>,
}

impl RenderArchetypeInfo {
    pub fn source(&self) -> &str {
        &self.source
    }
    pub fn answers(&self) -> &Option<Map> {
        &self.answers
    }
    pub fn switches(&self) -> &Option<HashSet<String>> {
        &self.switches
    }
    pub fn use_defaults(&self) -> &Option<HashSet<String>> {
        &self.use_defaults
    }
    pub fn use_defaults_all(&self) -> Option<bool> {
        self.use_defaults_all
    }
}

