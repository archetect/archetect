use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};

const DEFAULT_MODULES_DIRECTORIES: &str = "modules";

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ScriptEngine {
    Rhai,
    Lua,
}

impl Default for ScriptEngine {
    fn default() -> Self {
        ScriptEngine::Lua
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ScriptingConfig {
    #[serde(default)]
    pub engine: Option<ScriptEngine>,
    #[serde(default = "default_main")]
    pub main: Option<Utf8PathBuf>,
    #[serde(default = "default_modules")]
    modules: Utf8PathBuf,
}

impl ScriptingConfig {
    pub fn engine(&self) -> ScriptEngine {
        if let Some(engine) = &self.engine {
            return engine.clone();
        }
        // Infer from main script extension
        if let Some(main) = &self.main {
            if main.extension() == Some("rhai") {
                return ScriptEngine::Rhai;
            }
        }
        ScriptEngine::default()
    }

    pub fn main(&self) -> Utf8PathBuf {
        match &self.main {
            Some(path) => path.clone(),
            None => match self.engine() {
                ScriptEngine::Rhai => Utf8PathBuf::from("archetype.rhai"),
                ScriptEngine::Lua => Utf8PathBuf::from("archetype.lua"),
            },
        }
    }

    pub fn modules(&self) -> &Utf8Path {
        &self.modules
    }
}

impl Default for ScriptingConfig {
    fn default() -> Self {
        ScriptingConfig {
            engine: None,
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
