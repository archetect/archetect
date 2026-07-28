use chrono::TimeDelta;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ConfigurationUpdateSection {
    #[serde(skip_serializing_if = "Option::is_none")]
    force: Option<bool>,
    /// How often a moving ref re-checks the remote, in seconds. Default 1 day — content-addressing
    /// makes a re-check cheap (silent when the remote hasn't moved), so a short interval keeps
    /// moving refs fresh without churn.
    ///
    /// "Moving" means branches AND tags: only a bare commit rev is immutable and skips the probe
    /// entirely (see `RefPin` in archetect-git-cache). Tags move by design — the floating-major
    /// convention (`v1` tracking the latest v1.x.y) depends on it — so this interval governs how
    /// long a freshly published `v1` stays invisible to consumers.
    interval: i64,
    /// How long an unused materialized tree survives before `cache prune` reaps it, in seconds.
    /// Default 90 days.
    #[serde(default = "default_retention")]
    retention: i64,
}

fn default_retention() -> i64 {
    7_776_000 // 90 days
}

impl ConfigurationUpdateSection {
    pub fn force(&self) -> bool {
        self.force.unwrap_or_default()
    }

    pub fn interval(&self) -> TimeDelta {
        TimeDelta::try_seconds(self.interval).expect("Invalid Update Interval")
    }

    pub fn retention(&self) -> TimeDelta {
        TimeDelta::try_seconds(self.retention).expect("Invalid Retention")
    }
}

impl Default for ConfigurationUpdateSection {
    fn default() -> Self {
        ConfigurationUpdateSection {
            force: Default::default(),
            interval: 86_400,             // 1 day
            retention: default_retention(),
        }
    }
}
