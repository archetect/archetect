use std::path::PathBuf;
use crate::ArchetypeError;
use crate::system::SystemError;
use crate::util::SourceError;
use crate::config::{CatalogError, AnswerConfigError};

#[derive(Debug)]
pub enum ArchetectError {
    AnswerConfigError{ source: String, cause: AnswerConfigError },
    ArchetypeError(ArchetypeError),
    GenericError(String),
    RenderError(RenderError),
    SystemError(SystemError),
    SourceError(SourceError),
    CatalogError(CatalogError),
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

#[derive(Debug)]
pub enum RenderError {
    InvalidPathCharacters { source: PathBuf },
    PathRenderError{ source: PathBuf, error: crate::template_engine::Error, message: String },
    FileRenderError{ source: PathBuf, error: crate::template_engine::Error, message: String },

    FileRenderIOError { source: PathBuf, error: std::io::Error, message: String },
    StringRenderError { source: String, error: crate::template_engine::Error, message: String },
    IOError { error: std::io::Error, message: String },
}

impl From<std::io::Error> for RenderError {
    fn from(error: std::io::Error) -> Self {
        let message = error.to_string();
        RenderError::IOError { error, message }
    }
}
