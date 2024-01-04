use std::fs;
use std::path::Path;

use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};

use crate::actions::ArchetectAction;
use crate::archetype::archetype_manifest::RuntimeRequirements;
use crate::errors::CatalogError;

pub const CATALOG_FILE_NAMES: &[&str] = &["catalog.yaml", "catalog.yml"];

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CatalogManifest {
    requires: RuntimeRequirements,
    #[serde(with = "serde_yaml::with::singleton_map_recursive")]
    entries: Vec<ArchetectAction>,
}

impl CatalogManifest {
    pub fn new() -> CatalogManifest {
        CatalogManifest {
            requires: RuntimeRequirements::default(),
            entries: vec![],
        }
    }

    pub fn with_entries(mut self, entries: Vec<ArchetectAction>) -> Self {
        self.entries = entries;
        self
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

    pub fn entries(&self) -> &[ArchetectAction] {
        self.entries.as_slice()
    }

    pub fn entries_owned(&mut self) -> &mut Vec<ArchetectAction> {
        &mut self.entries
    }

    pub fn requirements(&self) -> &RuntimeRequirements {
        &self.requires
    }

    pub fn requires(&self) -> &RuntimeRequirements {
        &self.requires
    }
}

#[cfg(test)]
mod tests {
    use crate::actions::{RenderArchetypeInfo, RenderCatalogInfo, RenderGroupInfo};

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
                ArchetectAction::RenderCatalog {
                    description: "Java".to_owned(),
                    info: RenderCatalogInfo::new("~/projects/catalogs/java.yml"),
                },
            ],
        }
    }

    fn lang_group() -> ArchetectAction {
        ArchetectAction::RenderGroup {
            description: "Languages".to_owned(),
            info: RenderGroupInfo {
                entries: vec![rust_group(), python_group()],
            },
        }
    }

    fn rust_group() -> ArchetectAction {
        ArchetectAction::RenderGroup {
            description: "Rust".to_owned(),
            info: RenderGroupInfo {
                entries: vec![rust_cli_archetype(), rust_cli_workspace_archetype()],
            },
        }
    }

    fn rust_cli_archetype() -> ArchetectAction {
        ArchetectAction::RenderArchetype {
            description: "Rust CLI".to_owned(),
            info: RenderArchetypeInfo {
                source: "~/projects/test_archetypes/rust-cie".to_owned(),
                answers: None,
                switches: None,
                use_defaults: None,
                use_defaults_all: None,
            },
        }
    }

    fn rust_cli_workspace_archetype() -> ArchetectAction {
        ArchetectAction::RenderArchetype {
            description: "Rust CLI Workspace".to_owned(),
            info: RenderArchetypeInfo {
                source: "~/projects/test_archetypes/rust-cie".to_owned(),
                answers: None,
                switches: None,
                use_defaults: None,
                use_defaults_all: None,
            },
        }
    }

    fn python_group() -> ArchetectAction {
        ArchetectAction::RenderGroup {
            description: "Python".to_owned(),
            info: RenderGroupInfo {
                entries: vec![ArchetectAction::RenderArchetype {
                    description: "Python Service".to_owned(),
                    info: RenderArchetypeInfo {
                        source: "~/projects/python/python-service".to_owned(),
                        answers: None,
                        switches: None,
                        use_defaults: None,
                        use_defaults_all: None,
                    }
                }],
            },
        }
    }
}
