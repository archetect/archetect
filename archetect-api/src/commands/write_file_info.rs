use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WriteFileInfo {
    pub destination: String,
    pub contents: Vec<u8>,
    pub existing_file_policy: ExistingFilePolicy,
}

#[derive(Copy, Clone, Debug, Deserialize, Serialize)]
pub enum ExistingFilePolicy {
    Overwrite,
    Preserve,
    Prompt,
}
