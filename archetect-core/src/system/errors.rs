#[derive(Debug, thiserror::Error)]
pub enum SystemError {
    #[error("IO System Error: {source}")]
    IOError {
        #[from]
        source: std::io::Error,
    },
    #[error("System Error: {0}")]
    GenericError(String),
}

impl From<String> for SystemError {
    fn from(error: String) -> Self {
        SystemError::GenericError(error)
    }
}
