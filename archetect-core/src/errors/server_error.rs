#[derive(Debug, thiserror::Error)]
pub enum ArchetectServerError {
    #[error("Archetect Server IO Error: {0}")]
    ArchetectServerIoError(#[from] std::io::Error),
}
