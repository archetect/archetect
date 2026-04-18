use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Server-side config. Applies to `archetect server`. Every field is
/// optional — CLI flags and environment variables override what's set
/// here, and what's left falls through to the hardcoded defaults.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ConfigurationServerSection {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    host: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    port: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    tls: Option<ConfigurationServerTlsSection>,
}

impl ConfigurationServerSection {
    pub fn host(&self) -> Option<&str> {
        self.host.as_deref()
    }

    pub fn port(&self) -> Option<u16> {
        self.port
    }

    pub fn tls(&self) -> Option<&ConfigurationServerTlsSection> {
        self.tls.as_ref()
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ConfigurationServerTlsSection {
    cert: PathBuf,
    key: PathBuf,
    /// Optional CA cert for verifying client certificates (enables mTLS).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    client_ca: Option<PathBuf>,
}

impl ConfigurationServerTlsSection {
    pub fn cert(&self) -> &PathBuf {
        &self.cert
    }

    pub fn key(&self) -> &PathBuf {
        &self.key
    }

    pub fn client_ca(&self) -> Option<&PathBuf> {
        self.client_ca.as_ref()
    }
}
