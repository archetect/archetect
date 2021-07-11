use crate::config::{AnswerConfigError, CatalogError};
use crate::system::SystemError;
use crate::util::SourceError;
use crate::ArchetypeError;
use std::path::PathBuf;
use std::fmt::{Display, Formatter};
use std::error::Error;

#[derive(Debug, thiserror::Error)]
pub enum ArchetectError {
    #[error("Error in answer file `{path}`: {source}")]
    AnswerConfigError { path: String, source: AnswerConfigError },
    #[error(transparent)]
    ArchetypeError(#[from] ArchetypeError),
    #[error(transparent)]
    RenderError(#[from] RenderError),
    #[error(transparent)]
    SystemError(#[from] SystemError),
    #[error(transparent)]
    SourceError(#[from] SourceError),
    #[error(transparent)]
    CatalogError(#[from] CatalogError),
    #[error(transparent)]
    IoError(#[from] std::io::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    InvalidPathCharacters {
        path: PathBuf,
    },
    PathRenderError {
        path: PathBuf,
        source: crate::vendor::tera::Error,
    },
    FileRenderError {
        path: PathBuf,
        source: crate::vendor::tera::Error,
    },
    FileRenderIOError {
        path: PathBuf,
        source: std::io::Error,
    },
    StringRenderError {
        string: String,
        source: crate::vendor::tera::Error,
    },
    IOError {
        #[from]
        source: std::io::Error,
    },
}

impl Display for RenderError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            RenderError::InvalidPathCharacters { path } => {
                write!(f, "Invalid characters in path template `{:?}`", path)
            }
            RenderError::PathRenderError { path, source } => {
                write!(f, "Unable to render path `{:?}`: {}", path, extract_tera_message(source))
            }
            RenderError::FileRenderError { path, source } => {
                write!(f, "Unable to render file `{:?}`: {}", path, extract_tera_message(source))
            }
            RenderError::FileRenderIOError { path, source} => {
                write!(f, "Unable to render file `{:?}`: {}", path, source)
            }
            RenderError::StringRenderError { string, source } => {
                write!(f, "Unable to render string `{}`: {}", string, extract_tera_message(source))
            }
            RenderError::IOError { source } => {
                write!(f, "Rendering IO Error: {}", source)
            }
        }
    }
}

fn extract_tera_message(error: &crate::vendor::tera::Error) -> String {
    match error.source() {
        None => format!("{}", error),
        Some(source) => format!("{}", source)
    }
}
