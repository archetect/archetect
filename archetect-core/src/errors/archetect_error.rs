use std::env::VarError;
use crate::errors::answer_error::AnswerFileError;
use crate::errors::{ArchetypeError, RenderError, SourceError};
use crate::errors::{CatalogError, SystemError};
use rhai::EvalAltResult;
use shellexpand::LookupError;

#[derive(Debug, thiserror::Error)]
pub enum ArchetectError {
    #[error("Error in answer file `{path}`: {source}")]
    AnswerConfigError { path: String, source: AnswerFileError },
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
    CatalogError(#[from] CatalogError),
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    ShellEscape(#[from] LookupError<VarError>),
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
    #[error("Action \"{0}\" is not defined in configuration. Available actions: {1:?}.")]
    MissingAction(String, Vec<String>),
}
