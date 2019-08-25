use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct CatalogConfig {
    archetypes: Option<Vec<ArchetypeInfo>>,
}

impl CatalogConfig {
    pub fn new() -> Self {
        CatalogConfig { archetypes: None }
    }

    pub fn load<P: AsRef<Path>>(path: P) -> Result<CatalogConfig, CatalogConfigError> {
        let config_text = fs::read_to_string(path)?;
        toml::de::from_str::<CatalogConfig>(&config_text)
            .map_err(|e| CatalogConfigError::CatalogConfigTomlParseError(e))
    }

    pub fn add_archetype(&mut self, archetype: ArchetypeInfo) {
        let archetypes = self.archetypes.get_or_insert_with(|| vec![]);
        archetypes.push(archetype);
    }

    pub fn with_archetype(mut self, archetype: ArchetypeInfo) -> CatalogConfig {
        self.add_archetype(archetype);
        self
    }

    pub fn archetypes(&self) -> &[ArchetypeInfo] {
        self.archetypes.as_ref().map(|a| a.as_slice()).unwrap_or_default()
    }
}

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
pub struct ArchetypeInfo {
    description: String,
    source: String,
}

impl ArchetypeInfo {
    pub fn new(description: &str, location: &str) -> ArchetypeInfo {
        ArchetypeInfo {
            description: description.into(),
            source: location.into(),
        }
    }

    pub fn description(&self) -> &str {
        self.description.as_str()
    }

    pub fn source(&self) -> &str {
        self.source.as_str()
    }
}

#[cfg(test)]
mod tests {
    use crate::config::{ArchetypeInfo, CatalogConfig};
    use indoc::indoc;

    #[test]
    fn test_serialize_empty_catalog() {
        let catalog = CatalogConfig::new();
        assert_eq!(toml::ser::to_string(&catalog).unwrap(), "")
    }

    #[test]
    fn test_serialize_catalog_with_archetypes() {
        assert_eq!(
            toml::ser::to_string(
                &CatalogConfig::new().with_archetype(ArchetypeInfo::new("Rust CLI", "~/projects/rust-cli"))
            )
            .unwrap(),
            indoc! {
                r#"
                [[archetypes]]
                description = "Rust CLI"
                source = "~/projects/rust-cli"
                "#            
            }
        );

        assert_eq!(
            toml::ser::to_string(
                &CatalogConfig::new()
                    .with_archetype(ArchetypeInfo::new("Rust CLI", "~/projects/rust-cli"))
                    .with_archetype(ArchetypeInfo::new("Rust Rocket", "~/projects/rust-rocket"))
            )
            .unwrap(),
            indoc! {
                r#"
                [[archetypes]]
                description = "Rust CLI"
                source = "~/projects/rust-cli"

                [[archetypes]]
                description = "Rust Rocket"
                source = "~/projects/rust-rocket"
                "#
            }
        );
    }
}
