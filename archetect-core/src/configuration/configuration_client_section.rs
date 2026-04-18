use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Client-side config. Applies to `archetect connect`. Every field is
/// optional — CLI flags override what's set here.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ConfigurationClientSection {
    /// Default server endpoint (e.g. `https://archetect.example.com:8443`).
    /// When set, `archetect connect` without an explicit endpoint uses it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    endpoint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    connect: Option<ConfigurationClientConnectSection>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    keepalive: Option<ConfigurationClientKeepaliveSection>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    tls: Option<ConfigurationClientTlsSection>,
}

impl ConfigurationClientSection {
    pub fn endpoint(&self) -> Option<&str> {
        self.endpoint.as_deref()
    }
    pub fn connect(&self) -> Option<&ConfigurationClientConnectSection> {
        self.connect.as_ref()
    }
    pub fn keepalive(&self) -> Option<&ConfigurationClientKeepaliveSection> {
        self.keepalive.as_ref()
    }
    pub fn tls(&self) -> Option<&ConfigurationClientTlsSection> {
        self.tls.as_ref()
    }
}

/// Retry and timeout behavior for the initial gRPC connect.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ConfigurationClientConnectSection {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    timeout_secs: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    retries: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    backoff_base_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    max_backoff_secs: Option<u64>,
}

impl ConfigurationClientConnectSection {
    pub fn timeout_secs(&self) -> Option<u64> {
        self.timeout_secs
    }
    pub fn retries(&self) -> Option<u32> {
        self.retries
    }
    pub fn backoff_base_ms(&self) -> Option<u64> {
        self.backoff_base_ms
    }
    pub fn max_backoff_secs(&self) -> Option<u64> {
        self.max_backoff_secs
    }
}

/// HTTP/2 keepalive settings. Set `interval_secs: 0` to disable.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ConfigurationClientKeepaliveSection {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    interval_secs: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    timeout_secs: Option<u64>,
}

impl ConfigurationClientKeepaliveSection {
    pub fn interval_secs(&self) -> Option<u64> {
        self.interval_secs
    }
    pub fn timeout_secs(&self) -> Option<u64> {
        self.timeout_secs
    }
}

/// Client-side TLS settings. If this section is present (even empty), TLS
/// is enabled — equivalent to passing `--tls` on the CLI. Leaf fields are
/// all optional; empty trust and domain fall through to rustls defaults.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ConfigurationClientTlsSection {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    ca: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    client_cert: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    client_key: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    domain: Option<String>,
}

impl ConfigurationClientTlsSection {
    pub fn ca(&self) -> Option<&PathBuf> {
        self.ca.as_ref()
    }
    pub fn client_cert(&self) -> Option<&PathBuf> {
        self.client_cert.as_ref()
    }
    pub fn client_key(&self) -> Option<&PathBuf> {
        self.client_key.as_ref()
    }
    pub fn domain(&self) -> Option<&str> {
        self.domain.as_deref()
    }
}
