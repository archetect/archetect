use std::fs;
use std::path::Path;

#[derive(Debug)]
pub enum CatalogConfigError {
    CatalogConfigTomlParseError(toml::de::Error),
    IOError(std::io::Error),
}

impl From<std::io::Error> for CatalogConfigError {
    fn from(cause: std::io::Error) -> Self {
        CatalogConfigError::IOError(cause)
    }
}

impl From<toml::de::Error> for CatalogConfigError {
    fn from(cause: toml::de::Error) -> Self {
        CatalogConfigError::CatalogConfigTomlParseError(cause)
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CatalogInfo {
    description: String,
    source: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CatalogConfig {
    entries: Vec<CatalogConfigEntry>,
}

impl CatalogConfig {
    pub fn new() -> CatalogConfig {
        CatalogConfig { entries: vec![] }
    }

    pub fn entries(&self) -> &[CatalogConfigEntry] {
        self.entries.as_slice()
    }

    pub fn with_entry(mut self, entry: CatalogConfigEntry) -> CatalogConfig {
        self.add_entry(entry);
        self
    }

    pub fn add_entry(&mut self, entry: CatalogConfigEntry) {
        self.entries.push(entry);
    }

    pub fn load<P: AsRef<Path>>(path: P) -> Result<CatalogConfig, CatalogConfigError> {
        let config_text = fs::read_to_string(path)?;
        toml::de::from_str::<CatalogConfig>(&config_text).map_err(|e| CatalogConfigError::CatalogConfigTomlParseError(e))
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CatalogConfigEntry {
    #[serde(rename = "type")]
    pub entry_type: CatalogConfigEntryType,
    pub description: String,
    pub source: String,
}

impl CatalogConfigEntry {
    pub fn new<D: Into<String>, S: Into<String>>(
        description: D,
        source: S,
        entry_type: CatalogConfigEntryType,
    ) -> CatalogConfigEntry {
        CatalogConfigEntry {
            description: description.into(),
            source: source.into(),
            entry_type,
        }
    }

    pub fn description(&self) -> &str {
        self.description.as_str()
    }

    pub fn source(&self) -> &str {
        self.source.as_str()
    }

    pub fn entry_type(&self) -> &CatalogConfigEntryType {
        &self.entry_type
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum CatalogConfigEntryType {
    Catalog,
    Archetype,
}

//#[derive(Debug, Deserialize, Serialize, Clone)]
//pub struct ArchetypeEntry {
//    description: String,
//    source: String,
//}
//
//impl ArchetypeEntry {
//    pub fn new(description: &str, location: &str) -> ArchetypeEntry {
//        ArchetypeEntry {
//            description: description.into(),
//            source: location.into(),
//        }
//    }
//
//    pub fn description(&self) -> &str {
//        self.description.as_str()
//    }
//
//    pub fn source(&self) -> &str {
//        self.source.as_str()
//    }
//}

#[cfg(test)]
mod tests {
    use super::*;
//    use indoc::indoc;

    #[test]
    fn test_catalog_serialization() {
        let catalog = CatalogConfig::new()
            .with_entry(CatalogConfigEntry::new(
                "Rust CLI",
                "~/projects/archetypes/foo/",
                CatalogConfigEntryType::Archetype,
            ))
            .with_entry(CatalogConfigEntry::new(
                "Rust Archetypes",
                "http://www.example.com/catalog.toml",
                CatalogConfigEntryType::Catalog,
            ));

        println!("{}", toml::ser::to_string(&catalog).unwrap());
    }
}
