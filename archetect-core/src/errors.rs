use crate::config::{AnswerConfigError, CatalogError};
use crate::source::SourceError;
use crate::system::SystemError;
use crate::ArchetypeError;
use camino::Utf8PathBuf;
use rhai::EvalAltResult;
use std::fmt::{Display, Formatter};
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum ArchetectError {
    #[error("Error in answer file `{path}`: {source}")]
    AnswerConfigError { path: String, source: AnswerConfigError },
    #[error(transparent)]
    ArchetypeError(#[from] ArchetypeError),
    #[error(transparent)]
    RenderError(#[from] RenderError),
    #[error(transparent)]
    ScriptError(#[from] Box<EvalAltResult>),
    #[error(transparent)]
    SystemError(#[from] SystemError),
    #[error(transparent)]
    SourceError(#[from] SourceError),
    #[error(transparent)]
    SourceError2(#[from] crate::v2::source::SourceError),
    #[error(transparent)]
    CatalogError(#[from] CatalogError),
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(
        "Headless mode requires answers to be supplied for all variables, but no answer was supplied for the `{0}` \
    variable."
    )]
    HeadlessMissingAnswer(String),
    #[error("Headless mode attempted to use the default value for the `{identifier}` variable, however, {message}")]
    HeadlessInvalidDefault {
        identifier: String,
        default: String,
        message: String,
    },
    #[error(
        "Headless mode does not allow command line interaction, and requires a default value or answers to be set for this prompt style."
    )]
    HeadlessNoDefault,
    #[error("Error: {0}")]
    GeneralError(String),
}

#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    InvalidPathCharacters {
        path: PathBuf,
    },
    PathRenderError2 {
        path: PathBuf,
        source: minijinja::Error,
    },
    FileRenderIOError {
        path: Utf8PathBuf,
        source: std::io::Error,
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
            RenderError::PathRenderError2 { path, source } => {
                write!(f, "Unable to render path `{:?}`: {}", path, source)
            }
            RenderError::FileRenderIOError { path, source } => {
                write!(f, "Unable to render file `{:?}`: {}", path, source)
            }
            RenderError::IOError { source } => {
                write!(f, "Rendering IO Error: {}", source)
            }
        }
    }
}
