use camino::Utf8PathBuf;
use linked_hash_map::LinkedHashMap;
use serde::{Deserialize, Serialize};

pub use crate::archetype::archetype_manifest::requirements::RuntimeRequirements;
use crate::archetype::archetype_manifest::templating::TemplatingConfig;
use crate::errors::ArchetypeError;
use crate::manifest::{CatalogEntry, Manifest};

pub mod interface;
pub mod requirements;
pub mod templating;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ArchetypeManifest {
    description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    authors: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    languages: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    frameworks: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tags: Option<Vec<String>>,
    requires: RuntimeRequirements,
    #[serde(default = "TemplatingConfig::default")]
    templating: TemplatingConfig,
    /// Catalog entries (populated from unified Manifest).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    catalog: Option<LinkedHashMap<String, CatalogEntry>>,
    /// Declarative input contract for external tooling (web portals, MCP, docs).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    interface: Option<interface::ArchetypeInterface>,
}

impl ArchetypeManifest {
    /// Load an archetype manifest, delegating to `Manifest::load()` for unified file detection.
    pub fn load<P: Into<Utf8PathBuf>>(path: P) -> Result<ArchetypeManifest, ArchetypeError> {
        let manifest = Manifest::load(path)?;
        Ok(ArchetypeManifest::from(manifest))
    }

    pub fn description(&self) -> &str {
        self.description.as_str()
    }

    pub fn with_description(mut self, description: &str) -> ArchetypeManifest {
        self.description = description.to_string();
        self
    }

    pub fn add_author(&mut self, author: &str) {
        let authors = self.authors.get_or_insert_with(|| vec![]);
        authors.push(author.into());
    }

    pub fn with_author(mut self, author: &str) -> ArchetypeManifest {
        self.add_author(author);
        self
    }

    pub fn authors(&self) -> &[String] {
        self.authors.as_ref().map(|v| v.as_slice()).unwrap_or_default()
    }

    pub fn with_language(mut self, language: &str) -> ArchetypeManifest {
        self.add_language(language);
        self
    }

    pub fn add_language(&mut self, language: &str) {
        let languages = self.languages.get_or_insert_with(|| Vec::new());
        languages.push(language.to_owned());
    }

    pub fn languages(&self) -> &[String] {
        self.languages.as_ref().map(|r| r.as_slice()).unwrap_or_default()
    }

    pub fn with_tag(mut self, tag: &str) -> ArchetypeManifest {
        self.add_tag(tag);
        self
    }

    pub fn add_tag(&mut self, tag: &str) {
        let tags = self.tags.get_or_insert_with(|| Vec::new());
        tags.push(tag.to_owned());
    }

    pub fn tags(&self) -> &[String] {
        self.tags.as_ref().map(|r| r.as_slice()).unwrap_or_default()
    }

    pub fn with_framework(mut self, framework: &str) -> ArchetypeManifest {
        self.add_framework(framework);
        self
    }

    pub fn add_framework(&mut self, framework: &str) {
        let frameworks = self.frameworks.get_or_insert_with(|| Vec::new());
        frameworks.push(framework.to_owned());
    }

    pub fn frameworks(&self) -> &[String] {
        self.frameworks.as_ref().map(|r| r.as_slice()).unwrap_or_default()
    }

    pub fn requires(&self) -> &RuntimeRequirements {
        &self.requires
    }

    pub fn templating(&self) -> &TemplatingConfig {
        &self.templating
    }

    /// Returns catalog entries if this manifest declares any.
    pub fn catalog(&self) -> Option<&LinkedHashMap<String, CatalogEntry>> {
        self.catalog.as_ref()
    }

    /// Returns the declared interface contract, if any.
    pub fn interface(&self) -> Option<&interface::ArchetypeInterface> {
        self.interface.as_ref()
    }

    /// True if this manifest has non-empty catalog entries.
    pub fn has_catalog(&self) -> bool {
        self.catalog
            .as_ref()
            .map(|c| !c.is_empty())
            .unwrap_or(false)
    }
}

impl From<Manifest> for ArchetypeManifest {
    fn from(m: Manifest) -> Self {
        ArchetypeManifest {
            description: m.description,
            authors: if m.authors.is_empty() { None } else { Some(m.authors) },
            languages: if m.languages.is_empty() { None } else { Some(m.languages) },
            frameworks: if m.frameworks.is_empty() { None } else { Some(m.frameworks) },
            tags: if m.tags.is_empty() { None } else { Some(m.tags) },
            requires: m.requires,
            templating: m.templating,
            catalog: m.catalog,
            interface: m.interface,
        }
    }
}

#[cfg(test)]
mod tests {}
