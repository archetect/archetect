use std::path::PathBuf;

use camino::Utf8PathBuf;

use crate::errors::RequirementsError;

#[derive(Debug, thiserror::Error)]
pub enum ArchetypeError {
    #[error("The specified archetype is missing an archetype.yml or archetype.yaml file")]
    ArchetypeConfigMissing,
    #[error("The specified archetype config `{path}` does not exist")]
    ArchetypeConfigNotFound { path: PathBuf },
    #[error("The specified archetype manifest `{path}` does not exist")]
    ArchetypeManifestNotFound { path: Utf8PathBuf },
    #[error("Archetype by key `{key}` does not exist")]
    ArchetypeKeyNotFound { key: String },
    #[error("IO Error in Archetype: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Archetype Configuration Error in `{path}`: {source}")]
    YamlError { path: PathBuf, source: serde_yaml::Error },
    #[error("Archetype Manifest Syntax error in `{path}`: {source}")]
    ArchetypeManifestSyntaxError {
        path: Utf8PathBuf,
        source: serde_yaml::Error,
    },
    #[error("Operation was interrupted")]
    OperationInterrupted,
    #[error("Value is required")]
    ValueRequired,
    #[error("Archetype requirements failure:\n\n{0}")]
    RequirementsError(#[from] RequirementsError),
    #[error("Archetype Script Aborted")]
    ScriptAbortError,
}
