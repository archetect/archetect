use rhai::EvalAltResult;

#[derive(Debug, thiserror::Error)]
pub enum ArchetectIoDriverError {
    #[error("Aborted")]
    ClientDisconnected,
    #[error("Error: {message}")]
    ClientError { message: String },
    #[error("Script Channel Closed")]
    ScriptChannelClosed,
}

impl From<ArchetectIoDriverError> for Box<EvalAltResult> {
    fn from(value: ArchetectIoDriverError) -> Self {
        Box::new(EvalAltResult::ErrorSystem(value.to_string(), Box::new(value)))
    }
}
