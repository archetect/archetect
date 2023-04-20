#[derive(Clone, Debug)]
pub struct RuntimeContext {
    offline: bool,
    headless: bool,
    local: bool,
}

impl RuntimeContext {
    pub fn offline(&self) -> bool {
        self.offline
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
            local: false
        }
    }
}
