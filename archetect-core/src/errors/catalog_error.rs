use crate::errors::{RequirementsError, SourceError};
use camino::Utf8PathBuf;

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
    #[error("catalog.yml or catalog.yaml was not found in {0}")]
    NotFoundInDirectory(Utf8PathBuf),
    #[error("Catalog IO Error: {0}")]
    IOError(std::io::Error),
    #[error("Catalog Format Error: {0}")]
    YamlError(serde_yaml::Error),
    #[error("Catalog requirements failure:\n\n{0}")]
    RequirementsError(#[from] RequirementsError),
    #[error("Cancelled")]
    SelectionCancelled,
    #[error("Catalog Selection Error: {0}")]
    General(String),
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
