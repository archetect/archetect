use std::sync::{Arc, Mutex};
use std::sync::mpsc::Receiver;

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
    version: Version,
    updates: ConfigurationUpdateSection,
    locals: ConfigurationLocalsSection,
    io_driver: Box<dyn IoDriver>,
}

impl RuntimeContext {
    pub fn new<T: Into<Box<dyn IoDriver>>>(configuration: &Configuration, io_driver: T) -> RuntimeContext {
        RuntimeContext {
            inner: Arc::new(Inner {
                offline: configuration.offline(),
                headless: configuration.headless(),
                local: configuration.locals().enabled(),
                version: Version::parse(env!("CARGO_PKG_VERSION")).unwrap(),
                updates: configuration.updates().clone(),
                locals: configuration.locals().clone(),
                io_driver: io_driver.into(),
            })
        }
    }

    pub fn offline(&self) -> bool {
        self.inner.offline
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

    pub fn updates(&self) -> &ConfigurationUpdateSection {
        &self.inner.updates
    }

    pub fn locals(&self) -> &ConfigurationLocalsSection {
        &self.inner.locals
    }

    pub fn request(&self, command: CommandRequest) {
        self.inner.io_driver.send(command)
    }

    pub fn responses(&self) -> Arc<Mutex<Receiver<CommandResponse>>> {
        self.inner.io_driver.responses()
    }

    pub fn response(&self) -> CommandResponse {
        self.responses().lock().expect("Lock Error")
            .recv().expect("Receive Error")
    }
}
