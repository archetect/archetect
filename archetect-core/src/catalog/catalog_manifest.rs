use std::fs;
use std::path::Path;

use camino::Utf8PathBuf;

use crate::{Archetect, CacheCommand};
use crate::archetype::archetype_manifest::RuntimeRequirements;
use crate::configuration::{RenderArchetypeInfo, RenderCatalogInfo, RenderGroupInfo};
use crate::errors::{ArchetectError, CatalogError};
use crate::source::SourceCommand;

pub const CATALOG_FILE_NAMES: &[&str] = &["catalog.yaml", "catalog.yml"];

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CatalogManifest {
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

    pub fn with_entries(mut self, entries: Vec<CatalogEntry>) -> Self {
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
        #[serde(flatten)]
        info: RenderGroupInfo,
    },
    #[serde(rename = "catalog")]
    Catalog {
        description: String,
        #[serde(flatten)]
        info: RenderCatalogInfo,
    },
    #[serde(rename = "archetype")]
    Archetype {
        description: String,
        #[serde(flatten)]
        info: RenderArchetypeInfo,
    },
}

impl CatalogEntry {
    pub fn description(&self) -> &str {
        match self {
            CatalogEntry::Group { description, info: _ } => description.as_str(),
            CatalogEntry::Catalog { description, info: _ } => description.as_str(),
            CatalogEntry::Archetype { description, info: _} => description.as_str(),
        }
    }

    pub fn execute_cache_command(&self, archetect: &Archetect, command: CacheCommand) -> Result<(), ArchetectError> {
        match self {
            CatalogEntry::Group { description: _, info } => {
                for entry in &info.entries {
                    entry.execute_cache_command(archetect, command)?;
                }
            }
            CatalogEntry::Catalog { info, .. } => {
                let catalog = archetect.new_catalog(info.source())?;
                match command {
                    CacheCommand::Pull | CacheCommand::PullAll => {
                        if let Some(source) = catalog.source() {
                            source.execute(SourceCommand::Pull)?;
                        }
                        if let CacheCommand::PullAll = command {
                            for entry in catalog.entries() {
                                entry.execute_cache_command(archetect, command)?;
                            }
                        }
                    }
                    CacheCommand::Invalidate => {
                        if let Some(source) = catalog.source() {
                            source.execute(SourceCommand::Invalidate)?;
                        }
                    }
                    CacheCommand::View => unreachable!(),
                }
            }
            CatalogEntry::Archetype { description: _, info } => {
                let source = archetect.new_source(info.source())?;
                match command {
                    CacheCommand::Pull | CacheCommand::PullAll => {
                        source.execute(SourceCommand::Pull)?;
                    }
                    CacheCommand::Invalidate => {
                        source.execute(SourceCommand::Invalidate)?;
                    }
                    CacheCommand::View => unreachable!(),
                }
            }
        }

        Ok(())
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
                    info: RenderCatalogInfo::new("~/projects/catalogs/java.yml"),
                },
            ],
        }
    }

    fn lang_group() -> CatalogEntry {
        CatalogEntry::Group {
            description: "Languages".to_owned(),
            info: RenderGroupInfo {
                entries: vec![rust_group(), python_group()],
            },
        }
    }

    fn rust_group() -> CatalogEntry {
        CatalogEntry::Group {
            description: "Rust".to_owned(),
            info: RenderGroupInfo {
                entries: vec![rust_cli_archetype(), rust_cli_workspace_archetype()],
            },
        }
    }

    fn rust_cli_archetype() -> CatalogEntry {
        CatalogEntry::Archetype {
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

    fn rust_cli_workspace_archetype() -> CatalogEntry {
        CatalogEntry::Archetype {
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

    fn python_group() -> CatalogEntry {
        CatalogEntry::Group {
            description: "Python".to_owned(),
            info: RenderGroupInfo {
                entries: vec![CatalogEntry::Archetype {
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
