
#[derive(Debug)]
pub enum SystemError {
    IOError{ error: std::io::Error, message: Option<String> },
    GenericError(String),
}

impl From<std::io::Error> for SystemError {
    fn from(error: std::io::Error) -> Self {
        SystemError::IOError { error, message: None }
    }
}

impl From<String> for SystemError {
    fn from(error: String) -> Self {
        SystemError::GenericError(error)
    }
}