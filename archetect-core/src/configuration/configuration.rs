use crate::configuration::configuration_update_section::ConfigurationUpdateSection;
use git2;
use linked_hash_map::LinkedHashMap;
use rhai::Map;
use crate::configuration::configuration_local_section::ConfigurationLocalsSection;

use crate::catalog::{Catalog, CatalogEntry, CatalogManifest};

#[derive(Debug, Deserialize, Serialize)]
pub struct Configuration {
    #[serde(skip_serializing_if = "Option::is_none")]
    offline: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    headless: Option<bool>,
    answers: Map,
    #[serde(with = "serde_yaml::with::singleton_map_recursive")]
    catalogs: LinkedHashMap<String, Vec<CatalogEntry>>,
    updates: ConfigurationUpdateSection,
    locals: ConfigurationLocalsSection,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    switches: Vec<String>,
}

impl Configuration {
    pub fn headless(&self) -> bool {
        self.headless.unwrap_or_default()
    }
    pub fn offline(&self) -> bool {
        self.offline.unwrap_or_default()
    }
    pub fn updates(&self) -> &ConfigurationUpdateSection {
        &self.updates
    }

    pub fn locals(&self) -> &ConfigurationLocalsSection {
        &self.locals
    }

    pub fn answers(&self) -> &Map {
        &self.answers
    }

    pub fn catalogs(&self) -> &LinkedHashMap<String, Vec<CatalogEntry>> {
        &self.catalogs
    }

    pub fn catalog(&self) -> Catalog {
        let mut manifest = CatalogManifest::new();
        for (_key, entries) in self.catalogs() {
            for entry in entries.iter() {
                manifest.entries_owned().push(entry.to_owned());
            }
        }
        Catalog::new(manifest)
    }

    pub fn switches(&self) -> &[String] {
        &self.switches
    }

    pub fn to_yaml(&self) -> String {
        serde_yaml::to_string(&self)
            .expect("Unexpected error converting Configuration to yaml")
    }
}

impl Default for Configuration {
    fn default() -> Self {
        let mut catalogs = LinkedHashMap::new();
        catalogs.insert(
            "default".to_owned(),
            vec![CatalogEntry::Catalog {
                description: "Archetect".to_owned(),
                source: "https://github.com/archetect/archetect.catalog.git".to_owned(),
            }],
        );

        Self {
            headless: Default::default(),
            offline: Default::default(),
            updates: Default::default(),
            answers: derive_answers(),
            catalogs,
            locals: Default::default(),
            switches: Default::default(),
        }
    }
}

fn derive_answers() -> Map {
    let mut results = Map::new();

    if let Ok(config) = git2::Config::open_default() {
        let name = config.get_string("user.name");
        let email = config.get_string("user.email");

        if name.is_ok() {
            results.insert("author_name".to_owned().into(), name.as_ref().unwrap().into());
        }

        if email.is_ok() {
            results.insert("author_email".to_owned().into(), email.as_ref().unwrap().into());
        }

        if name.is_ok() && email.is_ok() {
            results.insert(
                "author_full".to_owned().into(),
                format!("{} <{}>", name.as_ref().unwrap(), email.as_ref().unwrap()).into(),
            );
        }
    }

    results
}

impl Configuration {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_defaults() -> anyhow::Result<()> {
        let configuration = Configuration::default();
        println!("{}", serde_yaml::to_string(&configuration)?);
        Ok(())
    }

    #[test]
    fn test_git2_config() -> anyhow::Result<()> {
        let config = git2::Config::open_default()?;

        let name = config.get_string("user.name")?;
        println!("{:?}", name);
        Ok(())
    }
}
