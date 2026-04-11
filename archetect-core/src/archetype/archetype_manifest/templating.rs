use serde::{Deserialize, Serialize};

/// Templating configuration declared in `archetype.yaml`.
///
/// Phase 1 of catalog-driven dependencies removed the `content` and
/// `includes` fields from this block. Both directories are now
/// standardized at fixed locations:
///
/// - Content directories are addressed by full root-relative paths in
///   `directory.render(path, context)` from the script — there is no
///   "content directory" prefix.
/// - The consumer's own `includes/` directory is automatically in the
///   include search list. Library archetypes contribute additional
///   include dirs via the catalog `library: true` mechanism.
///
/// What's left in this block is purely template-engine behavior:
/// undefined-variable resolution and Jinja-style whitespace controls.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct TemplatingConfig {
    /// Resolution policy for undefined context variables. `Lenient` (the
    /// default) drops them silently; `Strict` raises an error at render
    /// time.
    #[serde(default)]
    undefined: UndefinedMode,
    /// Strip the first newline after a `{% ... %}` block tag. Off by default.
    #[serde(default)]
    trim_blocks: bool,
    /// Strip leading whitespace on lines that contain only a block tag.
    /// Off by default.
    #[serde(default)]
    lstrip_blocks: bool,
}

impl TemplatingConfig {
    pub fn undefined(&self) -> UndefinedMode {
        self.undefined
    }

    pub fn trim_blocks(&self) -> bool {
        self.trim_blocks
    }

    pub fn lstrip_blocks(&self) -> bool {
        self.lstrip_blocks
    }
}

/// Resolution policy for undefined context variables in templates.
#[derive(Debug, Copy, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UndefinedMode {
    /// Undefined variables render as empty.
    #[default]
    Lenient,
    /// Undefined variables raise a render error.
    Strict,
}

#[cfg(test)]
mod tests {
    use super::*;
    use indoc::indoc;

    #[test]
    fn test_default_undefined_is_lenient() {
        let config = TemplatingConfig::default();
        assert_eq!(config.undefined(), UndefinedMode::Lenient);
    }

    #[test]
    fn test_default_trim_blocks_off() {
        let config = TemplatingConfig::default();
        assert!(!config.trim_blocks());
        assert!(!config.lstrip_blocks());
    }

    #[test]
    fn test_parse_full_templating_block() {
        let yaml = indoc! {r#"
            undefined: strict
            trim_blocks: true
            lstrip_blocks: true
        "#};
        let config: TemplatingConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.undefined(), UndefinedMode::Strict);
        assert!(config.trim_blocks());
        assert!(config.lstrip_blocks());
    }

    #[test]
    fn test_parse_minimal_templating_block_uses_defaults() {
        let yaml = "{}";
        let config: TemplatingConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.undefined(), UndefinedMode::Lenient);
        assert!(!config.trim_blocks());
        assert!(!config.lstrip_blocks());
    }

    #[test]
    fn test_legacy_chainable_undefined_behavior_errors_clearly() {
        // The old `chainable` variant of `undefined_behavior` is gone — Phase 1
        // is a hard cut, not a shim. A manifest still using it should fail to
        // parse cleanly so the user notices.
        let yaml = indoc! {r#"
            undefined: chainable
        "#};
        let result: Result<TemplatingConfig, _> = serde_yaml::from_str(yaml);
        assert!(
            result.is_err(),
            "expected parse error for legacy `chainable` value, got {:?}",
            result
        );
    }

    #[test]
    fn test_unknown_field_is_ignored() {
        // serde drops unknown keys by default, which preserves forward compat
        // for new fields added by future archetypes. Also covers the case of
        // an old archetype manifest still mentioning `content:` or
        // `includes:` — those are silently dropped.
        let yaml = indoc! {r#"
            content: "old_content"
            includes: "old_includes"
            undefined: strict
        "#};
        let config: TemplatingConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.undefined(), UndefinedMode::Strict);
    }
}
