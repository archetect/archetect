use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};

/// Scripting configuration declared in `archetype.yaml`.
///
/// Phase 1 of catalog-driven dependencies removed the `libraries` and
/// `modules` fields. Lua module locations are now standardized:
///
/// - The consumer's own `<root>/lib/` is on `package.path` automatically.
/// - External library code is declared via `catalog:` entries with
///   `library: true`.
///
/// What's left is the optional `main` field for archetypes that use a
/// non-default script filename.
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub struct ScriptingConfig {
    /// Override the main script filename. Default: `archetype.lua`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub main: Option<Utf8PathBuf>,
}

impl ScriptingConfig {
    pub fn main(&self) -> Utf8PathBuf {
        match &self.main {
            Some(path) => path.clone(),
            None => Utf8PathBuf::from("archetype.lua"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use indoc::indoc;

    #[test]
    fn test_parse_minimal_uses_defaults() {
        let yaml = "{}";
        let config: ScriptingConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.main(), Utf8PathBuf::from("archetype.lua"));
    }

    #[test]
    fn test_parse_main_override() {
        let yaml = indoc! {r#"
            main: "custom.lua"
        "#};
        let config: ScriptingConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.main(), Utf8PathBuf::from("custom.lua"));
    }

    #[test]
    fn test_legacy_libraries_field_silently_ignored() {
        // The old scripting.libraries field is gone. Existing manifests
        // mentioning it parse without error — serde drops unknown keys —
        // and the field has no effect.
        let yaml = indoc! {r#"
            main: "archetype.lua"
            libraries:
              - "old/path/one"
              - "old/path/two"
            modules: "old_modules"
        "#};
        let config: ScriptingConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.main(), Utf8PathBuf::from("archetype.lua"));
    }
}
