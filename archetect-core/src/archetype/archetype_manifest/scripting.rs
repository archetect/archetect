use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};

const DEFAULT_MAIN_SCRIPT: &str = "archetype.rhai";
const DEFAULT_MODULES_DIRECTORIES: &str = "modules";

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ScriptingConfig {
    #[serde(default = "default_main")]
    main: Utf8PathBuf,
    #[serde(default = "default_modules")]
    modules: Utf8PathBuf,
}

impl ScriptingConfig {
    pub fn main(&self) -> &Utf8Path {
        &self.main
    }

    pub fn modules(&self) -> &Utf8Path {
        &self.modules
    }
}

impl Default for ScriptingConfig {
    fn default() -> Self {
        ScriptingConfig {
            main: default_main(),
            modules: default_modules(),
        }
    }
}

fn default_main() -> Utf8PathBuf {
    Utf8PathBuf::from(DEFAULT_MAIN_SCRIPT)
}

fn default_modules() -> Utf8PathBuf {
    Utf8PathBuf::from(DEFAULT_MODULES_DIRECTORIES)
}
