use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};

use semver::Version;

use crate::archetype::archetype::Archetype;
use crate::catalog::Catalog;
use archetect_api::{CommandRequest, CommandResponse, IoDriver};
use archetect_terminal_io::TerminalIoDriver;

use crate::configuration::{Configuration, ConfigurationLocalsSection, ConfigurationUpdateSection};
use crate::errors::ArchetectError;
use crate::source::Source;
use crate::system::{RootedSystemLayout, SystemLayout};

#[derive(Clone, Debug)]
pub struct RuntimeContext {
    inner: Arc<Inner>,
}

#[derive(Debug)]
struct Inner {
    offline: bool,
    headless: bool,
    version: Version,
    updates: ConfigurationUpdateSection,
    locals: ConfigurationLocalsSection,
    io_driver: Box<dyn IoDriver>,
    layout: Box<dyn SystemLayout>,
}

pub struct RuntimeContextBuilder<'a> {
    configuration: Option<&'a Configuration>,
    layout: Option<Box<dyn SystemLayout>>,
    driver: Option<Box<dyn IoDriver>>,
}

impl<'a> RuntimeContextBuilder<'a> {
    pub fn with_layout<L: Into<Box<dyn SystemLayout>>>(mut self, layout: L) -> Self {
        self.layout = Some(layout.into());
        self
    }

    pub fn with_temp_layout(mut self) -> Result<Self, ArchetectError> {
        self.layout = Some(RootedSystemLayout::temp()?.into());
        Ok(self)
    }

    pub fn with_driver<D: Into<Box<dyn IoDriver>>>(mut self, driver: D) -> Self {
        self.driver = Some(driver.into());
        self
    }

    pub fn with_configuration(mut self, configuration: &'a Configuration) -> Self {
        self.configuration = Some(configuration);
        self
    }

    pub fn build(self) -> Result<RuntimeContext, ArchetectError> {
        let default_config = Configuration::default();
        let default_layout = RootedSystemLayout::dot_home()?;
        let configuration = self.configuration.unwrap_or(&default_config);
        let layout = self.layout.unwrap_or_else(|| default_layout.into());
        let driver = self.driver.unwrap_or_else(|| TerminalIoDriver::default().into());
        Ok(RuntimeContext::new(configuration, driver, layout))
    }
}

impl Default for RuntimeContextBuilder<'_> {
    fn default() -> Self {
        RuntimeContextBuilder {
            configuration: None,
            layout: None,
            driver: None,
        }
    }
}

impl RuntimeContext {
    pub fn new<T: Into<Box<dyn IoDriver>>, L: Into<Box<dyn SystemLayout>>>(
        configuration: &Configuration,
        driver: T,
        layout: L,
    ) -> RuntimeContext {
        RuntimeContext {
            inner: Arc::new(Inner {
                offline: configuration.offline(),
                headless: configuration.headless(),
                version: Version::parse(env!("CARGO_PKG_VERSION")).unwrap(),
                updates: configuration.updates().clone(),
                locals: configuration.locals().clone(),
                io_driver: driver.into(),
                layout: layout.into(),
            }),
        }
    }

    pub fn builder<'a>() -> RuntimeContextBuilder<'a> {
        RuntimeContextBuilder::default()
    }

    pub fn offline(&self) -> bool {
        self.inner.offline
    }

    pub fn headless(&self) -> bool {
        self.inner.headless
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
        self.responses()
            .lock()
            .expect("Lock Error")
            .recv()
            .expect("Receive Error")
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
