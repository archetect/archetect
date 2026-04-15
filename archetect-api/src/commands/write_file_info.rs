use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WriteFileInfo {
    pub destination: String,
    pub contents: Vec<u8>,
    pub existing_file_policy: ExistingFilePolicy,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum ExistingFilePolicy {
    Overwrite,
    Preserve,
    Prompt,
    /// Hard-fail the render if the destination file already exists.
    /// Useful for CI / idempotent pipelines where a collision should
    /// block the build rather than silently resolve either way.
    Error,
}
