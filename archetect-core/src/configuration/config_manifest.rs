use linked_hash_map::LinkedHashMap;
use rhai::Map;

use crate::v2::catalog::CatalogEntry;

#[derive(Debug, Deserialize, Serialize)]
pub struct ConfigManifest {
    offline: bool,
    answers: Map,
    catalogs: LinkedHashMap<String, Vec<CatalogEntry>>,
}

impl Default for ConfigManifest {
    fn default() -> Self {
        let mut catalogs = LinkedHashMap::new();
        catalogs.insert(
            "default".to_owned(),
            vec![CatalogEntry::Catalog {
                description: "Archetect".to_owned(),
                source: "https://github.co/archetect/catalog-archetect.git".to_owned(),
            }],
        );
        Self {
            offline: Default::default(),
            answers: Default::default(),
            catalogs,
        }
    }
}

impl ConfigManifest {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_defaults() -> anyhow::Result<()> {
        let configuration = ConfigManifest::default();
        println!("{}", serde_yaml::to_string(&configuration)?);
        Ok(())
    }
}
