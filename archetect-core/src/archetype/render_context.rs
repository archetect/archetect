use camino::{Utf8Path, Utf8PathBuf};
use std::collections::HashSet;

use archetect_api::{ContextMap, ContextValue};

use crate::actions::RenderArchetypeInfo;

#[derive(Clone, Debug)]
pub struct RenderContext {
    destination: Utf8PathBuf,
    answers: ContextMap,
    use_defaults: HashSet<String>,
    use_defaults_all: bool,
    switches: HashSet<String>,
    settings: ContextMap,
}

impl RenderContext {
    pub fn new<T: Into<Utf8PathBuf>>(destination: T, answers: ContextMap) -> RenderContext {
        RenderContext {
            destination: destination.into(),
            answers,
            use_defaults: Default::default(),
            use_defaults_all: false,
            switches: Default::default(),
            settings: Default::default(),
        }
    }

    pub fn with_archetype_info(mut self, info: &RenderArchetypeInfo) -> Self {
        if let Some(answers) = info.answers() {
            self.answers.extend(answers.clone());
        }
        if let Some(switches) = info.switches() {
            self = self.with_switches(switches.clone());
        }
        if let Some(use_defaults) = info.use_defaults() {
            self = self.with_use_defaults(use_defaults.clone());
        }
        if let Some(use_defaults_all) = info.use_defaults_all() {
            self = self.with_use_defaults_all(use_defaults_all);
        }
        self
    }

    pub fn answers(&self) -> &ContextMap {
        &self.answers
    }

    pub fn answers_owned(&self) -> ContextMap {
        self.answers.clone()
    }

    pub fn destination(&self) -> &Utf8Path {
        self.destination.as_path()
    }

    pub fn switches(&self) -> &HashSet<String> {
        &self.switches
    }

    /// Convert switches to a rhai::Array for the Rhai scripting engine.
    pub fn switches_as_rhai_array(&self) -> rhai::Array {
        self.switches.iter().map(|v| v.into()).collect()
    }

    pub fn with_switch<S: Into<String>>(mut self, switch: S) -> Self {
        self.switches.insert(switch.into());
        self
    }

    pub fn with_switches(mut self, switches: HashSet<String>) -> Self {
        self.set_switches(switches);
        self
    }

    pub fn set_switches(&mut self, switches: HashSet<String>) {
        self.switches = switches;
    }

    pub fn settings(&self) -> &ContextMap {
        &self.settings
    }

    pub fn with_settings(mut self, settings: ContextMap) -> Self {
        if let Some(ContextValue::Array(switches)) = settings.get("switches") {
            self.switches = switches
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
        }
        self.settings = settings;
        self
    }

    pub fn use_defaults(&self) -> &HashSet<String> {
        &self.use_defaults
    }

    /// Convert use_defaults to a rhai::Array for the Rhai scripting engine.
    pub fn use_defaults_as_rhai_array(&self) -> rhai::Array {
        self.use_defaults.iter().map(|v| v.into()).collect()
    }

    pub fn with_use_default<D: Into<String>>(mut self, default: D) -> Self {
        self.use_defaults.insert(default.into());
        self
    }

    pub fn with_use_defaults(mut self, defaults: HashSet<String>) -> Self {
        self.set_use_defaults(defaults);
        self
    }

    pub fn set_use_defaults(&mut self, use_defaults: HashSet<String>) {
        self.use_defaults = use_defaults;
    }

    pub fn use_defaults_all(&self) -> bool {
        self.use_defaults_all
    }

    pub fn with_use_defaults_all(mut self, value: bool) -> Self {
        self.set_use_defaults_all(value);
        self
    }

    pub fn set_use_defaults_all(&mut self, value: bool) {
        self.use_defaults_all = value;
    }
}
