use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::Receiver;

use camino::{Utf8Path, Utf8PathBuf};
use semver::Version;

use archetect_api::{CommandRequest, CommandResponse, IoDriver};

use crate::configuration::{Configuration, ConfigurationLocalsSection, ConfigurationUpdateSection};

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
    updates: ConfigurationUpdateSection,
    locals: ConfigurationLocalsSection,
    io_driver: Box<dyn IoDriver>,
}

impl RuntimeContext {
    pub fn new<T: Into<Box<dyn IoDriver>>>(configuration: &Configuration, mut switches: HashSet<String>, destination: Utf8PathBuf, io_driver: T) -> RuntimeContext {
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
                updates: configuration.updates().clone(),
                locals: configuration.locals().clone(),
                io_driver: io_driver.into(),
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

    pub fn updates(&self) -> &ConfigurationUpdateSection {
        &self.inner.updates
    }

    pub fn locals(&self) -> &ConfigurationLocalsSection {
        &self.inner.locals
    }

    pub fn request(&self, command: CommandRequest) {
        self.inner.io_driver.request(command)
    }

    pub fn responses(&self) -> Arc<Mutex<Receiver<CommandResponse>>> {
        self.inner.io_driver.responses()
    }
}
