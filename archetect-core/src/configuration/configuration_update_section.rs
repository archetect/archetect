use chrono::TimeDelta;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ConfigurationUpdateSection {
    #[serde(skip_serializing_if = "Option::is_none")]
    force: Option<bool>,
    interval: i64,
}

impl ConfigurationUpdateSection {
    pub fn force(&self) -> bool {
        self.force.unwrap_or_default()
    }

    pub fn interval(&self) -> TimeDelta {
        TimeDelta::try_seconds(self.interval).expect("Invalid Update Interval")
    }
}

impl Default for ConfigurationUpdateSection {
    fn default() -> Self {
        ConfigurationUpdateSection {
            force: Default::default(),
            interval: 604800,
        }
    }
}
