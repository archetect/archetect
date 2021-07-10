use crate::config::{AnswerConfigError, CatalogError};
use crate::system::SystemError;
use crate::util::SourceError;
use crate::ArchetypeError;
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum ArchetectError {
    #[error("Error in answer file `{path}`: {source}")]
    AnswerConfigError { path: String, source: AnswerConfigError },
    #[error(transparent)]
    ArchetypeError(ArchetypeError),
    #[error("{0}")]
    GenericError(String),
    #[error(transparent)]
    RenderError(RenderError),
    #[error(transparent)]
    SystemError(SystemError),
    #[error(transparent)]
    SourceError(SourceError),
    #[error(transparent)]
    CatalogError(CatalogError),
    #[error(transparent)]
    IoError(std::io::Error),
}

impl From<ArchetypeError> for ArchetectError {
    fn from(error: ArchetypeError) -> Self {
        ArchetectError::ArchetypeError(error)
    }
}

impl From<RenderError> for ArchetectError {
    fn from(error: RenderError) -> Self {
        ArchetectError::RenderError(error)
    }
}

impl From<String> for ArchetectError {
    fn from(error: String) -> Self {
        ArchetectError::GenericError(error)
    }
}

impl From<SystemError> for ArchetectError {
    fn from(error: SystemError) -> Self {
        ArchetectError::SystemError(error)
    }
}

impl From<SourceError> for ArchetectError {
    fn from(error: SourceError) -> Self {
        ArchetectError::SourceError(error)
    }
}

impl From<CatalogError> for ArchetectError {
    fn from(error: CatalogError) -> Self {
        ArchetectError::CatalogError(error)
    }
}

impl From<std::io::Error> for ArchetectError {
    fn from(error: std::io::Error) -> ArchetectError {
        ArchetectError::IoError(error)
    }
}

// TODO: Implement Display by hand
#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error("Invalid characters in path template {path}")]
    InvalidPathCharacters {
        path: PathBuf,
    },
    #[error("Unable to render path `{path}`")]
    PathRenderError {
        path: PathBuf,
        source: crate::vendor::tera::Error,
    },
    #[error("Unable to render contents of `{path}`")]
    FileRenderError {
        path: PathBuf,
        source: crate::vendor::tera::Error,
    },
    #[error("Unable to render contents of `{path}`: {source}")]
    FileRenderIOError {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("Unable to render `{string}`.")]
    StringRenderError {
        string: String,
        source: crate::vendor::tera::Error,
    },
    #[error("Rendering IO Error: {source}")]
    IOError {
        #[from]
        source: std::io::Error,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test() {
        let error = RenderError::FileRenderError {
            path: PathBuf::from("/some/path"),
            source: crate::vendor::tera::Error::filter_not_found("train_case"),
        };
        println!("{}", error);
    }
}
