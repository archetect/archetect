use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};

const DEFAULT_MODULES_DIRECTORIES: &str = "modules";

/// Scripting configuration declared in `archetype.yaml`.
///
/// Phase 5 of the ATL evolution plan added `libraries` — a list of
/// additional directories prepended to Lua's `package.path` so that
/// `require("foo.bar")` can locate shared library code that ships with
/// the archetype, separate from the author's per-archetype `modules/`.
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub struct ScriptingConfig {
    #[serde(default = "default_main")]
    pub main: Option<Utf8PathBuf>,
    #[serde(default = "default_modules")]
    modules: Utf8PathBuf,
    /// Additional directories appended to Lua's `package.path` for
    /// `require()` resolution. Paths are relative to the archetype root.
    #[serde(default)]
    libraries: Vec<Utf8PathBuf>,
}

impl ScriptingConfig {
    pub fn main(&self) -> Utf8PathBuf {
        match &self.main {
            Some(path) => path.clone(),
            None => Utf8PathBuf::from("archetype.lua"),
        }
    }

    pub fn modules(&self) -> &Utf8Path {
        &self.modules
    }

    pub fn libraries(&self) -> &[Utf8PathBuf] {
        &self.libraries
    }
}

fn default_main() -> Option<Utf8PathBuf> {
    None
}

fn default_modules() -> Utf8PathBuf {
    Utf8PathBuf::from(DEFAULT_MODULES_DIRECTORIES)
}

#[cfg(test)]
mod tests {
    use super::*;
    use indoc::indoc;

    #[test]
    fn test_default_libraries_empty() {
        let config = ScriptingConfig::default();
        assert!(config.libraries().is_empty());
    }

    #[test]
    fn test_parse_libraries() {
        let yaml = indoc! {r#"
            main: "archetype.lua"
            libraries:
              - "lib/utils"
              - "lib/codegen"
        "#};
        let config: ScriptingConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            config.libraries(),
            &[
                Utf8PathBuf::from("lib/utils"),
                Utf8PathBuf::from("lib/codegen"),
            ]
        );
    }

    #[test]
    fn test_parse_minimal_uses_defaults() {
        let yaml = "{}";
        let config: ScriptingConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(config.libraries().is_empty());
        assert_eq!(config.modules(), Utf8Path::new("modules"));
    }
}
