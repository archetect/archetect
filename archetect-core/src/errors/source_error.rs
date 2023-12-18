#[derive(Debug, thiserror::Error)]
pub enum SourceError {
    #[error("Unsupported source: `{0}`")]
    SourceUnsupported(String),
    #[error("Failed to find a default 'develop', 'main', or 'master' branch.")]
    NoDefaultBranch,
    #[error("Source not found: `{0}`")]
    SourceNotFound(String),
    #[error("Invalid Source Path: `{0}`")]
    SourceInvalidPath(String),
    #[error("Invalid Source Encoding: `{0}`")]
    SourceInvalidEncoding(String),
    #[error("Remote Source Error: `{0}`")]
    RemoteSourceError(String),
    #[error("Remote Source is not cached, and Archetect was run in offline mode: `{0}`")]
    OfflineAndNotCached(String),
    #[error("Source IO Error: `{0}`")]
    IoError(std::io::Error),
    #[error("Git Error: `{0}`")]
    GitError(#[from] git2::Error),
    #[error("Source does not contain either an Archetype or Catalog")]
    UnknownSourceContent,
}

impl From<std::io::Error> for SourceError {
    fn from(error: std::io::Error) -> SourceError {
        SourceError::IoError(error)
    }
}
