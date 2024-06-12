use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct ServerConfiguration {
    host: String,
    port: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    banner: Option<String>,
}

impl Default for ServerConfiguration {
    fn default() -> Self {
        ServerConfiguration {
            host: "0.0.0.0".to_string(),
            port: 8080,
            banner: None,
        }
    }
}

impl ServerConfiguration {
    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn host(&self) -> &str {
        self.host.as_str()
    }

    pub fn banner(&self) -> Option<&String> {
        self.banner.as_ref()
    }
}
