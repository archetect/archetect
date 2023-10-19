use semver::{Version, VersionReq};
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum RequirementsError {
    #[error("Error Deserializing Requirements File `{path}`: {cause}")]
    DeserializationError { path: PathBuf, cause: serde_yaml::Error },
    #[error(
        "Incompatible Archetect Version `{0}`. This archetype or one of it's components requires version {1}. \
     \n\nPlease install the latest version: cargo install archetect --force"
    )]
    ArchetectVersion(Version, VersionReq),
    #[error("IO Error Reading Requirements File `{0}`.")]
    IoError(std::io::Error),
}

impl From<std::io::Error> for RequirementsError {
    fn from(error: std::io::Error) -> Self {
        RequirementsError::IoError(error)
    }
}
