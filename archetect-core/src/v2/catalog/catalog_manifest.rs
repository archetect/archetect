use crate::v2::archetype::manifest::RuntimeRequirements;
use crate::v2::runtime::context::RuntimeContext;
use crate::v2::source::{Source, SourceError};
use crate::ArchetypeError;
use camino::Utf8PathBuf;
use rhai::Map;
use std::fs;
use std::path::Path;

pub const CATALOG_FILE_NAME: &str = "catalog.yml";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CatalogManifest {
    #[serde(default = "RuntimeRequirements::default")]
    requirements: RuntimeRequirements,
    entries: Vec<CatalogEntry>,
}

impl CatalogManifest {
    pub fn new() -> CatalogManifest {
        CatalogManifest {
            requirements: RuntimeRequirements::default(),
            entries: vec![],
        }
    }

    pub fn load(source: Source) -> Result<CatalogManifest, CatalogError> {
        // TODO: Support both yml and yaml extensions
        let catalog_path = match source {
            Source::LocalFile { path } => path,
            Source::RemoteHttp { url: _, path } => path,
            Source::RemoteGit {
                url: _,
                path,
                gitref: _,
            } => path.join(CATALOG_FILE_NAME),
            Source::LocalDirectory { path } => path.join(CATALOG_FILE_NAME),
        };

        if !catalog_path.exists() {
            return Err(CatalogError::NotFound(catalog_path));
        }

        let catalog = fs::read_to_string(&catalog_path)?;
        let catalog = serde_yaml::from_str(&catalog)?;
        Ok(catalog)
    }

    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), CatalogError> {
        let yaml = serde_yaml::to_string(&self)?;
        fs::write(path, yaml)?;
        Ok(())
    }

    pub fn entries(&self) -> &[CatalogEntry] {
        self.entries.as_slice()
    }

    pub fn entries_owned(self) -> Vec<CatalogEntry> {
        self.entries
    }

    pub fn requirements(&self) -> &RuntimeRequirements {
        &self.requirements
    }

    pub fn check_requirements(&self, runtime_context: &RuntimeContext) -> Result<(), ArchetypeError> {
        self.requirements.check_requirements(runtime_context)
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

#[derive(Debug, thiserror::Error)]
pub enum CatalogError {
    #[error("Catalog File is Empty")]
    EmptyCatalog,
    #[error("Selected Catalog Group is Empty")]
    EmptyGroup,
    #[error("Invalid Catalog Source: {0}")]
    SourceError(SourceError),
    #[error("Catalog not found: {0}")]
    NotFound(Utf8PathBuf),
    #[error("Catalog IO Error: {0}")]
    IOError(std::io::Error),
    #[error("Catalog Format Error: {0}")]
    YamlError(serde_yaml::Error),
}

impl From<std::io::Error> for CatalogError {
    fn from(e: std::io::Error) -> Self {
        CatalogError::IOError(e)
    }
}

impl From<serde_yaml::Error> for CatalogError {
    fn from(e: serde_yaml::Error) -> Self {
        CatalogError::YamlError(e)
    }
}

impl From<SourceError> for CatalogError {
    fn from(cause: SourceError) -> Self {
        CatalogError::SourceError(cause)
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
            requirements: None,
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
