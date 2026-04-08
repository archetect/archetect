use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};

const DEFAULT_MODULES_DIRECTORIES: &str = "modules";

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ScriptingConfig {
    #[serde(default = "default_main")]
    pub main: Option<Utf8PathBuf>,
    #[serde(default = "default_modules")]
    modules: Utf8PathBuf,
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
}

impl Default for ScriptingConfig {
    fn default() -> Self {
        ScriptingConfig {
            main: default_main(),
            modules: default_modules(),
        }
    }
}

fn default_main() -> Option<Utf8PathBuf> {
    None
}

fn default_modules() -> Utf8PathBuf {
    Utf8PathBuf::from(DEFAULT_MODULES_DIRECTORIES)
}
