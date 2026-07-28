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

    /// Force a re-probe of every moving ref this run, skipping the `interval` gate.
    ///
    /// The CLI reaches this through figment (`-U`/`--force-update` → `updates.force`), but an
    /// EMBEDDER has no figment layer — it builds a `Configuration` in code — so without a setter
    /// there was no way for one to offer its own `-U`. Prova embeds archetect-core for
    /// `prova init`, and that is exactly the gap: publish an archetype, run `prova init`, and the
    /// moved tag stays invisible for up to `interval` (a day by default) with no way to say
    /// "check now".
    pub fn with_force(mut self, value: bool) -> Self {
        self.set_force(value);
        self
    }

    /// In-place form, so a caller holding the section behind `&mut` can set just this field.
    /// [`Configuration::with_force_update`] needs it: rebuilding the section from `Default` to reach
    /// the builder would quietly reset a figment-loaded `interval`/`retention`.
    pub fn set_force(&mut self, value: bool) {
        self.force = Some(value);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn force_defaults_off_and_sets() {
        let s = ConfigurationUpdateSection::default();
        assert!(!s.force(), "force is off unless asked for");
        assert!(s.clone().with_force(true).force());
    }

    /// The bug this guards: reaching the builder by replacing the section with `Default` resets
    /// `interval`/`retention` as a side effect of setting one unrelated flag. An embedder that
    /// lowered `interval` to keep moving tags fresh would silently get the 1-day default back the
    /// moment it asked for a forced update.
    #[test]
    fn setting_force_preserves_the_other_fields() {
        let mut s = ConfigurationUpdateSection::default();
        s.interval = 60;
        s.retention = 120;

        let s = s.with_force(true);

        assert!(s.force());
        assert_eq!(s.interval().num_seconds(), 60, "interval survived");
        assert_eq!(s.retention().num_seconds(), 120, "retention survived");
    }
}
