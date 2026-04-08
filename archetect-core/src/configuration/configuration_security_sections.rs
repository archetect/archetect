use serde::{Deserialize, Serialize};

/// Controls how `archetect.shell.run` and `archetect.shell.capture` are gated.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ShellExecPolicy {
    /// Hard block — shell exec attempts always fail. Used by MCP mode.
    Forbidden,
    /// Default — every shell exec call prompts the user, showing the exact command.
    /// Headless mode without explicit `Allowed` results in a hard failure.
    Prompt,
    /// Shell exec is allowed unconditionally. Set by `--allow-exec` or
    /// the `ARCHETECT_ALLOW_EXEC` environment variable.
    Allowed,
}

impl Default for ShellExecPolicy {
    fn default() -> Self {
        ShellExecPolicy::Prompt
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub struct ConfigurationSecuritySection {
    /// Backwards-compatible bool: when set in user config, maps to
    /// `Allowed` (true) or `Prompt` (false).
    #[serde(skip_serializing_if = "Option::is_none")]
    allow_exec: Option<bool>,
    /// Explicit override that takes precedence over `allow_exec` when present.
    #[serde(skip_serializing_if = "Option::is_none")]
    shell_exec_policy: Option<ShellExecPolicy>,
}

impl ConfigurationSecuritySection {
    /// Resolved policy: explicit `shell_exec_policy` wins, then legacy `allow_exec`,
    /// then the default (`Prompt`).
    pub fn shell_exec_policy(&self) -> ShellExecPolicy {
        if let Some(policy) = self.shell_exec_policy {
            return policy;
        }
        match self.allow_exec {
            Some(true) => ShellExecPolicy::Allowed,
            Some(false) => ShellExecPolicy::Prompt,
            None => ShellExecPolicy::default(),
        }
    }

    pub fn with_shell_exec_policy(mut self, policy: ShellExecPolicy) -> Self {
        self.shell_exec_policy = Some(policy);
        self
    }

    pub fn set_shell_exec_policy(&mut self, policy: ShellExecPolicy) {
        self.shell_exec_policy = Some(policy);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_policy_is_prompt() {
        let security = ConfigurationSecuritySection::default();
        assert_eq!(security.shell_exec_policy(), ShellExecPolicy::Prompt);
    }

    #[test]
    fn legacy_allow_exec_true_maps_to_allowed() {
        let yaml = "allow_exec: true";
        let security: ConfigurationSecuritySection = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(security.shell_exec_policy(), ShellExecPolicy::Allowed);
    }

    #[test]
    fn legacy_allow_exec_false_maps_to_prompt() {
        let yaml = "allow_exec: false";
        let security: ConfigurationSecuritySection = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(security.shell_exec_policy(), ShellExecPolicy::Prompt);
    }

    #[test]
    fn explicit_policy_overrides_legacy() {
        let yaml = indoc::indoc! {r#"
            allow_exec: true
            shell_exec_policy: forbidden
        "#};
        let security: ConfigurationSecuritySection = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(security.shell_exec_policy(), ShellExecPolicy::Forbidden);
    }

    #[test]
    fn explicit_policy_forbidden() {
        let yaml = "shell_exec_policy: forbidden";
        let security: ConfigurationSecuritySection = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(security.shell_exec_policy(), ShellExecPolicy::Forbidden);
    }

    #[test]
    fn explicit_policy_allowed() {
        let yaml = "shell_exec_policy: allowed";
        let security: ConfigurationSecuritySection = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(security.shell_exec_policy(), ShellExecPolicy::Allowed);
    }

    #[test]
    fn with_shell_exec_policy_builder() {
        let security = ConfigurationSecuritySection::default()
            .with_shell_exec_policy(ShellExecPolicy::Forbidden);
        assert_eq!(security.shell_exec_policy(), ShellExecPolicy::Forbidden);
    }
}
