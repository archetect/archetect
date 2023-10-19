#[derive(Debug, PartialEq, thiserror::Error)]
pub enum AnswerFileError {
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

impl From<serde_yaml::Error> for AnswerFileError {
    fn from(error: serde_yaml::Error) -> Self {
        AnswerFileError::ParseError(error.to_string())
    }
}

impl From<std::io::Error> for AnswerFileError {
    fn from(_: std::io::Error) -> Self {
        // TODO: Distinguish between missing and other errors
        AnswerFileError::MissingError
    }
}
