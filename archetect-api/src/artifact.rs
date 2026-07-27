//! What a render produced, as the caller sees it.
//!
//! A remote caller cannot inspect the destination directory to work out what it
//! got — and the interesting names are computed by the script from answers
//! (`prefix` + `suffix` → `customer-service.zip`), so they cannot be guessed
//! either. Completion therefore carries an explicit manifest.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactKind {
    /// A packaged archive written into the destination.
    Archive,
    /// A repository the render published to.
    Repository,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Artifact {
    pub kind: ArtifactKind,
    /// Destination-relative path, for artifacts the client wrote to disk.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Locator for artifacts that live outside the destination — a repo URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
}

impl Artifact {
    pub fn archive<P: Into<String>>(path: P) -> Self {
        Artifact { kind: ArtifactKind::Archive, path: Some(path.into()), uri: None }
    }

    pub fn repository<U: Into<String>>(uri: U) -> Self {
        Artifact { kind: ArtifactKind::Repository, path: None, uri: Some(uri.into()) }
    }

    /// How this artifact reads in a one-line report.
    pub fn locator(&self) -> &str {
        self.path
            .as_deref()
            .or(self.uri.as_deref())
            .unwrap_or("<unknown>")
    }
}
