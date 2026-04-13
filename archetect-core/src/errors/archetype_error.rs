use std::path::PathBuf;

use camino::Utf8PathBuf;

use crate::errors::{RequirementsError, SourceError};

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
    #[error("Error creating directory `{path}`: {source}")]
    DirectoryError{
        path: Utf8PathBuf,
        source: std::io::Error,
    },
    #[error(transparent)]
    SourceError(#[from] SourceError),
    #[error("Operation was interrupted")]
    OperationInterrupted,
    #[error("Value is required")]
    ValueRequired,
    #[error("Archetype requirements failure:\n\n{0}")]
    RequirementsError(#[from] RequirementsError),
    #[error("Archetype Script Aborted")]
    ScriptAbortError,
    /// User cancelled an interactive prompt (Esc / Ctrl-C). Propagates
    /// through nested render chains so a cancel inside a composed
    /// component also kills the parent archetype. The top-level CLI
    /// handler translates this into a clean, quiet exit (no stack
    /// trace / error dump).
    #[error("Cancelled")]
    PromptAborted,
}
