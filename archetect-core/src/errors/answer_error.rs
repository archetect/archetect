use crate::config::answers::Rule;
use pest::error::Error as PestError;

#[derive(Debug, PartialEq, thiserror::Error)]
pub enum AnswerConfigError {
    #[error("Error parsing answer config: {0}")]
    ParseError(String),
    #[error("Answer file does not exist")]
    MissingError,
    #[error("Provided answer file is not a supported answer file format")]
    InvalidFileType,
    #[error("Provided answer file must be structured as a JSON Object")]
    InvalidJsonAnswerFileStructure,
    #[error("Provided answer file must be structured as a YAML Object")]
    InvalidYamlAnswerFileStructure,
    #[error("Provided answer file must resolve to a Rhai Object")]
    InvalidRhaiAnswerFileStructure,
}

impl From<serde_yaml::Error> for AnswerConfigError {
    fn from(error: serde_yaml::Error) -> Self {
        AnswerConfigError::ParseError(error.to_string())
    }
}

impl From<std::io::Error> for AnswerConfigError {
    fn from(_: std::io::Error) -> Self {
        // TODO: Distinguish between missing and other errors
        AnswerConfigError::MissingError
    }
}
#[derive(Debug, PartialEq)]
pub enum AnswerParseError {
    PestError(PestError<Rule>),
}

impl From<PestError<Rule>> for AnswerParseError {
    fn from(error: PestError<Rule>) -> Self {
        AnswerParseError::PestError(error)
    }
}
