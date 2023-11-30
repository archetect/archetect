use std::fs;
use std::path::Path;

use camino::Utf8PathBuf;
use rhai::Map;

use crate::errors::CatalogError;
use crate::archetype::archetype_manifest::RuntimeRequirements;

pub const CATALOG_FILE_NAMES: &[&str] = &["catalog.yaml", "catalog.yml"];

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CatalogManifest {
    #[serde(default = "RuntimeRequirements::default")]
    requires: RuntimeRequirements,
    #[serde(with = "serde_yaml::with::singleton_map_recursive")]
    entries: Vec<CatalogEntry>,
}

impl CatalogManifest {
    pub fn new() -> CatalogManifest {
        CatalogManifest {
            requires: RuntimeRequirements::default(),
            entries: vec![],
        }
    }

    pub fn load<P: Into<Utf8PathBuf>>(path: P) -> Result<CatalogManifest, CatalogError> {
        let mut path = path.into();
        if path.is_dir() {
            for candidate in CATALOG_FILE_NAMES {
                let config_file = path.join(candidate);
                if config_file.exists() {
                    path = config_file;
                }
            }
        }

        if path.is_dir() {
            Err(CatalogError::NotFoundInDirectory(path))
        } else if !path.exists() {
            Err(CatalogError::NotFound(path))
        } else {
            let catalog = fs::read_to_string(&path)?;
            return match serde_yaml::from_str::<CatalogManifest>(&catalog) {
                Ok(catalog) => Ok(catalog),
                Err(source) => Err(CatalogError::YamlError(source)),
            };
        }
    }

    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), CatalogError> {
        let yaml = serde_yaml::to_string(&self)?;
        fs::write(path, yaml)?;
        Ok(())
    }

    pub fn entries(&self) -> &[CatalogEntry] {
        self.entries.as_slice()
    }

    pub fn entries_owned(&mut self) -> &mut Vec<CatalogEntry> {
        &mut self.entries
    }

    pub fn requirements(&self) -> &RuntimeRequirements {
        &self.requires
    }

    pub fn requires(&self) -> &RuntimeRequirements {
        &self.requires
    }
}

impl Default for CatalogManifest {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum CatalogEntry {
    #[serde(rename = "group")]
    Group {
        description: String,
        entries: Vec<CatalogEntry>,
    },
    #[serde(rename = "catalog")]
    Catalog { description: String, source: String },
    #[serde(rename = "archetype")]
    Archetype {
        description: String,
        source: String,
        answers: Option<Map>,
    },
}

impl CatalogEntry {
    pub fn description(&self) -> &str {
        match self {
            CatalogEntry::Group {
                description,
                entries: _,
            } => description.as_str(),
            CatalogEntry::Catalog { description, source: _ } => description.as_str(),
            CatalogEntry::Archetype {
                description,
                source: _,
                answers: _,
            } => description.as_str(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        let catalog = prototype_catalog();
        let yaml = serde_yaml::to_string(&catalog).unwrap();
        println!("{}", yaml);
    }

    #[test]
    fn test_catalog_group() {
        let group = lang_group();

        let yaml = serde_yaml::to_string(&group).unwrap();
        println!("{}", yaml);
    }

    fn prototype_catalog() -> CatalogManifest {
        CatalogManifest {
            requires: RuntimeRequirements::default(),
            entries: vec![
                lang_group(),
                CatalogEntry::Catalog {
                    description: "Java".to_owned(),
                    source: "~/projects/catalogs/java.yml".to_owned(),
                },
            ],
        }
    }

    fn lang_group() -> CatalogEntry {
        CatalogEntry::Group {
            description: "Languages".to_owned(),
            entries: vec![rust_group(), python_group()],
        }
    }

    fn rust_group() -> CatalogEntry {
        CatalogEntry::Group {
            description: "Rust".to_owned(),
            entries: vec![rust_cli_archetype(), rust_cli_workspace_archetype()],
        }
    }

    fn rust_cli_archetype() -> CatalogEntry {
        CatalogEntry::Archetype {
            description: "Rust CLI".to_owned(),
            source: "~/projects/test_archetypes/rust-cie".to_owned(),
            answers: None,
        }
    }

    fn rust_cli_workspace_archetype() -> CatalogEntry {
        CatalogEntry::Archetype {
            description: "Rust CLI Workspace".to_owned(),
            source: "~/projects/test_archetypes/rust-cie".to_owned(),
            answers: None,
        }
    }

    fn python_group() -> CatalogEntry {
        CatalogEntry::Group {
            description: "Python".to_owned(),
            entries: vec![CatalogEntry::Archetype {
                description: "Python Service".to_owned(),
                source: "~/projects/python/python-service".to_owned(),
                answers: None,
            }],
        }
    }
}