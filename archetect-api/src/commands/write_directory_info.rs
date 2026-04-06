use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WriteDirectoryInfo {
    pub path: String,
}
