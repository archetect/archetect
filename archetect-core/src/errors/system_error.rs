#[derive(Debug, thiserror::Error)]
pub enum SystemError {
    #[error("IO System Error: {source}")]
    IOError {
        #[from]
        source: std::io::Error,
    },
    #[error("System Error: {0}")]
    HomeDirectoryNotFound(String),
}
