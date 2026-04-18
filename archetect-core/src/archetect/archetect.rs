use std::sync::Arc;

use semver::Version;

use archetect_api::{ClientMessage, IoError, ScriptIoHandle, ScriptMessage};
use archetect_terminal_io::TerminalScriptIoHandle;

use crate::archetype::archetype::Archetype;
use crate::configuration::Configuration;
use crate::errors::ArchetectError;
use crate::source::Source;
use crate::system::{RootedSystemLayout, SystemLayout, XdgSystemLayout};

#[derive(Clone, Debug)]
pub struct Archetect {
    inner: Arc<Inner>,
}

#[derive(Debug)]
struct Inner {
    version: Version,
    io_driver: Box<dyn ScriptIoHandle>,
    layout: Box<dyn SystemLayout>,
    configuration: Configuration,
}

pub struct ArchetectBuilder {
    configuration: Option<Configuration>,
    layout: Option<Box<dyn SystemLayout>>,
    driver: Option<Box<dyn ScriptIoHandle>>,
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

    pub fn with_driver<D: Into<Box<dyn ScriptIoHandle>>>(mut self, driver: D) -> Self {
        self.driver = Some(driver.into());
        self
    }

    pub fn with_configuration(mut self, configuration: Configuration) -> Self {
        self.configuration = Some(configuration);
        self
    }

    pub fn build(self) -> Result<Archetect, ArchetectError> {
        let configuration = self.configuration.unwrap_or(Configuration::default());
        let layout = match self.layout {
            Some(layout) => layout,
            None => XdgSystemLayout::new()?.into(),
        };
        let driver = self.driver.unwrap_or_else(|| TerminalScriptIoHandle::default().into());
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
    pub fn new<T: Into<Box<dyn ScriptIoHandle>>, L: Into<Box<dyn SystemLayout>>>(
        configuration: Configuration,
        driver: T,
        layout: L,
    ) -> Archetect {
        Archetect {
            inner: Arc::new(Inner {
                version: Version::parse(env!("CARGO_PKG_VERSION"))
                    .expect("CARGO_PKG_VERSION is always valid semver"),
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

    pub fn is_dry_run(&self) -> bool {
        self.inner.configuration.dry_run()
    }

    pub fn version(&self) -> &Version {
        &self.inner.version
    }

    pub fn layout(&self) -> &Box<dyn SystemLayout> {
        &self.inner.layout
    }

    pub fn request(&self, command: ScriptMessage) -> Result<(), IoError> {
        self.inner.io_driver.send(command)
    }

    pub fn configuration(&self) -> &Configuration {
        &self.inner.configuration
    }

    pub fn response(&self) -> Result<ClientMessage, IoError> {
        self.inner.io_driver.receive()
    }

    pub fn new_archetype(&self, path: &str) -> Result<Archetype, ArchetectError> {
        let source = self.new_source(path)?;
        let archetype = Archetype::new(self.clone(), source)?;
        Ok(archetype)
    }

    pub fn new_source(&self, path: &str) -> Result<Source, ArchetectError> {
        let source = Source::new(self.clone(), path)?;
        Ok(source)
    }

    pub fn check(&self) -> Result<(), ArchetectError> {
        crate::check::check_all(self)?;
        Ok(())
    }
}
