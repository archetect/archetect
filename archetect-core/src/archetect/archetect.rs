use std::sync::Arc;

use semver::Version;

use archetect_api::{CommandRequest, CommandResponse, IoDriver};
use archetect_terminal_io::TerminalIoDriver;

use crate::archetype::archetype::Archetype;
use crate::catalog::{Catalog, CatalogManifest};
use crate::configuration::Configuration;
use crate::errors::ArchetectError;
use crate::source::Source;
use crate::system::{RootedSystemLayout, SystemLayout};

#[derive(Clone, Debug)]
pub struct Archetect {
    inner: Arc<Inner>,
}

#[derive(Debug)]
struct Inner {
    version: Version,
    io_driver: Box<dyn IoDriver>,
    layout: Box<dyn SystemLayout>,
    configuration: Configuration,
}

pub struct ArchetectBuilder {
    configuration: Option<Configuration>,
    layout: Option<Box<dyn SystemLayout>>,
    driver: Option<Box<dyn IoDriver>>,
}

impl ArchetectBuilder {
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

    pub fn with_configuration(mut self, configuration: Configuration) -> Self {
        self.configuration = Some(configuration);
        self
    }

    pub fn build(self) -> Result<Archetect, ArchetectError> {
        let configuration = self.configuration.unwrap_or(Configuration::default());
        let default_layout = RootedSystemLayout::dot_home()?;
        let layout = self.layout.unwrap_or_else(|| default_layout.into());
        let driver = self.driver.unwrap_or_else(|| TerminalIoDriver::default().into());
        Ok(Archetect::new(configuration, driver, layout))
    }
}

impl Default for ArchetectBuilder {
    fn default() -> Self {
        ArchetectBuilder {
            configuration: None,
            layout: None,
            driver: None,
        }
    }
}

impl Archetect {
    pub fn new<T: Into<Box<dyn IoDriver>>, L: Into<Box<dyn SystemLayout>>>(
        configuration: Configuration,
        driver: T,
        layout: L,
    ) -> Archetect {
        Archetect {
            inner: Arc::new(Inner {
                version: Version::parse(env!("CARGO_PKG_VERSION")).unwrap(),
                io_driver: driver.into(),
                layout: layout.into(),
                configuration,
            }),
        }
    }

    pub fn builder() -> ArchetectBuilder {
        ArchetectBuilder::default()
    }

    pub fn is_offline(&self) -> bool {
        self.inner.configuration.offline()
    }

    pub fn is_headless(&self) -> bool {
        self.inner.configuration.headless()
    }

    pub fn version(&self) -> &Version {
        &self.inner.version
    }

    pub fn layout(&self) -> &Box<dyn SystemLayout> {
        &self.inner.layout
    }

    pub fn request(&self, command: CommandRequest) {
        self.inner.io_driver.send(command)
    }

    pub fn configuration(&self) -> &Configuration {
        &self.inner.configuration
    }

    pub fn response(&self) -> CommandResponse {
        self.inner.io_driver.responses()
            .lock()
            .expect("Lock Error")
            .recv()
            .expect("Receive Error")
    }

    pub fn new_archetype(&self, path: &str, force_pull: bool) -> Result<Archetype, ArchetectError> {
        let source = Source::create(&self, path, force_pull)?;
        let archetype = Archetype::new(self.clone(), &source)?;
        Ok(archetype)
    }

    pub fn new_catalog(&self, path: &str, force_pull: bool) -> Result<Catalog, ArchetectError> {
        let source = Source::create(&self, path, force_pull)?;
        let catalog = Catalog::load(self.clone(), &source)?;
        Ok(catalog)
    }


    pub fn catalog(&self) -> Catalog {
        let mut manifest = CatalogManifest::new();
        for (_key, entries) in self.configuration().catalogs() {
            for entry in entries.iter() {
                manifest.entries_owned().push(entry.to_owned());
            }
        }
        Catalog::new(self.clone(), manifest)
    }
}
