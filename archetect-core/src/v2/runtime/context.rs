use std::collections::HashSet;
use std::sync::Arc;

use camino::{Utf8Path, Utf8PathBuf};
use semver::Version;

use crate::configuration::Configuration;

#[derive(Clone, Debug)]
pub struct RuntimeContext {
    inner: Arc<Inner>,
}

#[derive(Debug)]
struct Inner {
    offline: bool,
    headless: bool,
    local: bool,
    switches: HashSet<String>,
    version: Version,
    destination: Utf8PathBuf,
}


impl RuntimeContext {
    pub fn new(configuration: &Configuration, mut switches: HashSet<String>, destination: Utf8PathBuf) -> RuntimeContext {
        for switch in configuration.switches() {
            switches.insert(switch.to_string());
        }
        RuntimeContext {
            inner: Arc::new(Inner {
                offline: configuration.offline(),
                headless: configuration.headless(),
                local: configuration.locals().enabled(),
                switches,
                version: Version::parse(env!("CARGO_PKG_VERSION")).unwrap(),
                destination,
            })
        }
    }

    pub fn offline(&self) -> bool {
        self.inner.offline
    }

    pub fn switches(&self) -> &HashSet<String> {
        &self.inner.switches
    }

    pub fn switch_enabled<T: AsRef<str>>(&self, switch: T) -> bool {
        self.inner.switches.contains(switch.as_ref())
    }

    pub fn headless(&self) -> bool {
        self.inner.headless
    }

    pub fn local(&self) -> bool {
        self.inner.local
    }

    pub fn archetect_version(&self) -> &Version {
        &self.inner.version
    }

    pub fn destination(&self) -> &Utf8Path {
        &self.inner.destination
    }
}
