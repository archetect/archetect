use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};

const DEFAULT_CONTENT_DIRECTORY: &str = ".";
const DEFAULT_TEMPLATES_DIRECTORY: &str = "templates";
const DEFAULT_INCLUDES_DIRECTORY: &str = "includes";

/// Templating configuration declared in `archetype.yaml`.
///
/// Phase 5 of the ATL evolution plan replaced the legacy
/// `undefined_behavior` enum (a MiniJinja-era artifact) with the simpler
/// `undefined: lenient | strict` field, and added explicit slots for the
/// includes directory and Jinja-style whitespace controls.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TemplatingConfig {
    #[serde(default = "default_content_directory")]
    content: Utf8PathBuf,
    #[serde(default = "default_templates_directory")]
    templates: Utf8PathBuf,
    /// Directory that `{% include "..." %}` resolves paths against. Default
    /// is `includes`. If the directory is absent and no template uses an
    /// include, no error is raised.
    #[serde(default = "default_includes_directory")]
    includes: Utf8PathBuf,
    /// Resolution policy for undefined context variables. `Lenient` (the
    /// default) drops them silently; `Strict` will raise an error at render
    /// time once Phase 6 lands. Phase 5 only adds the field — strict mode
    /// is not yet wired into the render path.
    #[serde(default)]
    undefined: UndefinedMode,
    /// Strip the first newline after a `{% ... %}` block tag. Off by default.
    /// Wired up in Phase 7.
    #[serde(default)]
    trim_blocks: bool,
    /// Strip leading whitespace on lines that contain only a block tag.
    /// Off by default. Wired up in Phase 7.
    #[serde(default)]
    lstrip_blocks: bool,
}

impl TemplatingConfig {
    pub fn content_directory(&self) -> &Utf8Path {
        &self.content
    }

    pub fn templates_directory(&self) -> &Utf8Path {
        &self.templates
    }

    pub fn includes_directory(&self) -> &Utf8Path {
        &self.includes
    }

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

impl Default for TemplatingConfig {
    fn default() -> Self {
        TemplatingConfig {
            content: default_content_directory(),
            templates: default_templates_directory(),
            includes: default_includes_directory(),
            undefined: UndefinedMode::default(),
            trim_blocks: false,
            lstrip_blocks: false,
        }
    }
}

fn default_content_directory() -> Utf8PathBuf {
    Utf8PathBuf::from(DEFAULT_CONTENT_DIRECTORY)
}

fn default_templates_directory() -> Utf8PathBuf {
    Utf8PathBuf::from(DEFAULT_TEMPLATES_DIRECTORY)
}

fn default_includes_directory() -> Utf8PathBuf {
    Utf8PathBuf::from(DEFAULT_INCLUDES_DIRECTORY)
}

/// Resolution policy for undefined context variables in templates.
#[derive(Debug, Copy, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UndefinedMode {
    /// Undefined variables render as empty. Matches Phase 1's nil-renders-empty
    /// behavior. This is the default.
    #[default]
    Lenient,
    /// Undefined variables raise a render error. Implemented in Phase 6.
    Strict,
}

#[cfg(test)]
mod tests {
    use super::*;
    use indoc::indoc;

    #[test]
    fn test_default_includes_directory_is_includes() {
        let config = TemplatingConfig::default();
        assert_eq!(config.includes_directory(), Utf8Path::new("includes"));
    }

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
            content: "."
            templates: "templates"
            includes: "partials"
            undefined: strict
            trim_blocks: true
            lstrip_blocks: true
        "#};
        let config: TemplatingConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.includes_directory(), Utf8Path::new("partials"));
        assert_eq!(config.undefined(), UndefinedMode::Strict);
        assert!(config.trim_blocks());
        assert!(config.lstrip_blocks());
    }

    #[test]
    fn test_parse_minimal_templating_block_uses_defaults() {
        let yaml = "{}";
        let config: TemplatingConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.includes_directory(), Utf8Path::new("includes"));
        assert_eq!(config.undefined(), UndefinedMode::Lenient);
        assert!(!config.trim_blocks());
        assert!(!config.lstrip_blocks());
    }

    #[test]
    fn test_parse_undefined_lenient_explicit() {
        let yaml = indoc! {r#"
            undefined: lenient
        "#};
        let config: TemplatingConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.undefined(), UndefinedMode::Lenient);
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
        // for new fields added by future archetypes.
        let yaml = indoc! {r#"
            includes: "shared"
            something_new: 42
        "#};
        let config: TemplatingConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.includes_directory(), Utf8Path::new("shared"));
    }
}
