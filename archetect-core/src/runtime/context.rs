use std::sync::{Arc, Mutex};
use std::sync::mpsc::Receiver;

use semver::Version;

use archetect_api::{CommandRequest, CommandResponse, IoDriver};
use crate::archetype::archetype::Archetype;
use crate::catalog::Catalog;

use crate::configuration::{Configuration, ConfigurationLocalsSection, ConfigurationUpdateSection};
use crate::errors::ArchetectError;
use crate::source::Source;
use crate::system::SystemLayout;

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
    layout: Box<dyn SystemLayout>,
}

impl RuntimeContext {
    pub fn new<T: Into<Box<dyn IoDriver>>, L: Into<Box<dyn SystemLayout>>>(configuration: &Configuration, driver: T, layout: L) -> RuntimeContext {
        RuntimeContext {
            inner: Arc::new(Inner {
                offline: configuration.offline(),
                headless: configuration.headless(),
                local: configuration.locals().enabled(),
                version: Version::parse(env!("CARGO_PKG_VERSION")).unwrap(),
                updates: configuration.updates().clone(),
                locals: configuration.locals().clone(),
                io_driver: driver.into(),
                layout: layout.into(),
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

    pub fn layout(&self) -> &Box<dyn SystemLayout> {
        &self.inner.layout
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

    pub fn new_archetype(&self, path: &str) -> Result<Archetype, ArchetectError> {
        let source = Source::detect(&self, path)?;
        let archetype = Archetype::new(&source)?;
        Ok(archetype)
    }

    pub fn new_catalog(&self, path: &str) -> Result<Catalog, ArchetectError> {
        let source = Source::detect(&self, path)?;
        let catalog = Catalog::load(&source)?;
        Ok(catalog)
    }
}
