use std::collections::HashSet;

#[derive(Clone, Debug)]
pub struct RuntimeContext {
    offline: bool,
    headless: bool,
    local: bool,
    switches: HashSet<String>,
}

impl RuntimeContext {
    pub fn offline(&self) -> bool {
        self.offline
    }

    pub fn enable_switch<S: Into<String>>(&mut self, switch: S) {
        self.switches.insert(switch.into());
    }

    pub fn switches(&self) -> &HashSet<String> {
        &self.switches
    }

    pub fn switch_enabled<T: AsRef<str>>(&self, switch: T) -> bool {
        self.switches.contains(switch.as_ref())
    }

    pub fn set_offline(&mut self, offline: bool) {
        self.offline = offline;
    }

    pub fn headless(&self) -> bool {
        self.headless
    }

    pub fn set_headless(&mut self, headless: bool) {
        self.headless = headless;
    }

    pub fn local(&self) -> bool {
        self.local
    }

    pub fn set_local(&mut self, local: bool) {
        self.local = local;
    }
}

impl Default for RuntimeContext {
    fn default() -> Self {
        RuntimeContext {
            offline: false,
            headless: false,
            local: false,
            switches: HashSet::new(),
        }
    }
}
