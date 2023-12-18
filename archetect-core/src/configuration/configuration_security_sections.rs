
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ConfigurationSecuritySection {
    #[serde(skip_serializing_if = "Option::is_none")]
    allow_exec: Option<bool>,
}

impl ConfigurationSecuritySection {
    pub fn allow_exec(&self) -> Option<bool> {
       self.allow_exec.clone()
    }
}

impl Default for ConfigurationSecuritySection {
    fn default() -> Self {
        ConfigurationSecuritySection {
            allow_exec: None
        }
    }
}