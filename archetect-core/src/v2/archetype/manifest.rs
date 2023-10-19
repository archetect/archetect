mod requirements;
mod scripting;
mod templating;

use crate::errors::ArchetypeError;
pub use crate::v2::archetype::manifest::requirements::RuntimeRequirements;
use crate::v2::archetype::manifest::scripting::ScriptingConfig;
use crate::v2::archetype::manifest::templating::TemplatingConfig;
use camino::Utf8PathBuf;
use linked_hash_map::LinkedHashMap;
use std::fs;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ArchetypeManifest {
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    authors: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    languages: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    frameworks: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tags: Option<Vec<String>>,
    requires: RuntimeRequirements,
    #[serde(skip_serializing_if = "Option::is_none")]
    components: Option<LinkedHashMap<String, String>>,
    #[serde(default = "ScriptingConfig::default")]
    scripting: ScriptingConfig,
    #[serde(default = "TemplatingConfig::default")]
    templating: TemplatingConfig,
}

impl ArchetypeManifest {
    pub fn load<P: Into<Utf8PathBuf>>(path: P) -> Result<ArchetypeManifest, ArchetypeError> {
        let mut path = path.into();
        if path.is_dir() {
            let candidates = vec!["archetype.yml", "archetype.yaml"];
            for candidate in candidates {
                let config_file = path.join(candidate);
                if config_file.exists() {
                    path = config_file;
                }
            }
        }
        if path.is_dir() {
            Err(ArchetypeError::ArchetypeConfigMissing)
        } else if !path.exists() {
            Err(ArchetypeError::ArchetypeManifestNotFound { path })
        } else {
            let config = fs::read_to_string(&path)?;
            return match serde_yaml::from_str::<ArchetypeManifest>(&config) {
                Ok(config) => Ok(config),
                Err(source) => Err(ArchetypeError::ArchetypeManifestSyntaxError { path, source }),
            };
        }
    }

    pub fn with_description(mut self, description: &str) -> ArchetypeManifest {
        self.description = Some(description.into());
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

    pub fn with_component(mut self, key: &str, source: &str) -> ArchetypeManifest {
        self.components
            .get_or_insert_with(|| LinkedHashMap::new())
            .insert(key.into(), source.into());
        // self.archetypes.insert(key.into(), source.into());
        self
    }

    pub fn components(&self) -> Option<&LinkedHashMap<String, String>> {
        self.components.as_ref()
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

    pub fn scripting(&self) -> &ScriptingConfig {
        &self.scripting
    }

    pub fn templating(&self) -> &TemplatingConfig {
        &self.templating
    }
}

#[cfg(test)]
mod tests {
    // use super::*;

    // #[test]
    // fn test_serialize_to_yaml() {
    //     let config = ArchetypeManifest::default()
    //         .with_description("Simple REST Service")
    //         .with_language("Java")
    //         .with_framework("Spring")
    //         .with_framework("Hessian")
    //         .with_tag("Service")
    //         .with_tag("REST")
    //         .with_component("rust-service", "git:/rust-foo")
    //         .with_component("java-service", "git:/java-foo");
    //
    //     let output = serde_yaml::to_string(&config).unwrap();
    //     println!("{}", output);
    // }
}
