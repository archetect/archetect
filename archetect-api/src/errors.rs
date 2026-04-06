use std::fmt;

#[derive(Clone, Debug)]
pub enum IoError {
    ScriptChannelClosed,
    ClientDisconnected,
    ClientError { message: String },
}

impl fmt::Display for IoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IoError::ScriptChannelClosed => write!(f, "Script channel closed"),
            IoError::ClientDisconnected => write!(f, "Client disconnected"),
            IoError::ClientError { message } => write!(f, "Client error: {}", message),
        }
    }
}

impl std::error::Error for IoError {}
