use serde::{Deserialize, Serialize};

const DEFAULT_HOST: &str = "0.0.0.0";
const DEFAULT_SERVICE_PORT: u16 = 8080;
const DEFAULT_MANAGEMENT_PORT: u16 = 8081;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerSettings {
    host: String,
    service: ServiceSettings,
    management: ManagementSettings,
}

impl ServerSettings {
    pub fn host(&self) -> &str {
        self.host.as_str()
    }

    pub fn service(&self) -> &ServiceSettings {
        &self.service
    }

    pub fn service_mut(&mut self) -> &mut ServiceSettings {
        &mut self.service
    }

    pub fn management(&self) -> &ManagementSettings {
        &self.management
    }
}

impl Default for ServerSettings {
    fn default() -> Self {
        ServerSettings {
            host: String::from(DEFAULT_HOST),
            service: Default::default(),
            management: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceSettings {
    port: u16,
}

impl ServiceSettings {
    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn set_port(&mut self, port: u16) -> &mut ServiceSettings {
        self.port = port;
        self
    }

    pub fn with_port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }
}

impl Default for ServiceSettings {
    fn default() -> Self {
        ServiceSettings {
            port: DEFAULT_SERVICE_PORT,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagementSettings {
    port: u16,
}

impl ManagementSettings {
    pub fn port(&self) -> u16 {
        self.port
    }
}

impl Default for ManagementSettings {
    fn default() -> Self {
        ManagementSettings {
            port: DEFAULT_MANAGEMENT_PORT,
        }
    }
}
