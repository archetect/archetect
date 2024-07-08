use std::sync::Arc;

use semver::Version;

use archetect_api::{ClientMessage, ScriptIoHandle, ScriptMessage};
use archetect_terminal_io::TerminalIoDriver;

use crate::actions::ArchetectAction;
use crate::archetype::archetype::Archetype;
use crate::archetype::render_context::RenderContext;
use crate::catalog::{Catalog, CatalogManifest};
use crate::configuration::Configuration;
use crate::errors::{ArchetectError, ArchetectIoDriverError};
use crate::source::Source;
use crate::system::{RootedSystemLayout, SystemLayout};

#[derive(Clone, Debug)]
pub struct Archetect {
    inner: Arc<Inner>,
}

#[derive(Debug)]
struct Inner {
    version: Version,
    script_io_handle: Box<dyn ScriptIoHandle>,
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
    pub fn new<T: Into<Box<dyn ScriptIoHandle>>, L: Into<Box<dyn SystemLayout>>>(
        configuration: Configuration,
        driver: T,
        layout: L,
    ) -> Archetect {
        Archetect {
            inner: Arc::new(Inner {
                version: Version::parse(env!("CARGO_PKG_VERSION")).unwrap(),
                script_io_handle: driver.into(),
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

    pub fn configuration(&self) -> &Configuration {
        &self.inner.configuration
    }

    pub fn request(&self, command: ScriptMessage) -> Result<(), ArchetectIoDriverError> {
        match self.inner.script_io_handle.send(command) {
            None => Err(ArchetectIoDriverError::ScriptChannelClosed),
            Some(_) => Ok(()),
        }
    }

    pub fn receive(&self) -> Result<ClientMessage, ArchetectIoDriverError> {
        match self.inner.script_io_handle.receive() {
            None => Err(ArchetectIoDriverError::ClientDisconnected),
            Some(message) => match message {
                ClientMessage::Error(message) => Err(ArchetectIoDriverError::ClientError { message }),
                ClientMessage::Abort => Err(ArchetectIoDriverError::ClientDisconnected),
                other => Ok(other),
            },
        }
    }

    pub fn new_archetype(&self, path: &str) -> Result<Archetype, ArchetectError> {
        let source = self.new_source(path)?;
        let archetype = Archetype::new(self.clone(), source)?;
        Ok(archetype)
    }

    pub fn new_catalog(&self, path: &str) -> Result<Catalog, ArchetectError> {
        let source = self.new_source(path)?;
        let catalog = Catalog::load(self.clone(), source)?;
        Ok(catalog)
    }

    pub fn new_source(&self, path: &str) -> Result<Source, ArchetectError> {
        let source = Source::new(self.clone(), path)?;
        Ok(source)
    }

    pub fn execute_action<A: Into<String>>(
        &self,
        action: A,
        render_context: RenderContext,
    ) -> Result<(), ArchetectError> {
        let action = action.into();
        match self.configuration().action(&action) {
            None => {
                return Err(ArchetectError::MissingAction(
                    action.to_owned(),
                    self.configuration()
                        .actions()
                        .keys()
                        .map(|v| v.to_string())
                        .collect::<Vec<String>>(),
                ));
            }
            Some(command) => {
                match command {
                    ArchetectAction::RenderGroup { info, .. } => {
                        let catalog = Catalog::new(
                            self.clone(),
                            CatalogManifest::new().with_entries(info.actions().clone()),
                        );
                        catalog.render(render_context)?;
                    }
                    ArchetectAction::RenderCatalog { info, .. } => {
                        let catalog = self.new_catalog(info.source())?;
                        catalog.check_requirements()?;
                        catalog.render(render_context)?;
                    }
                    ArchetectAction::RenderArchetype { info, .. } => {
                        let archetype = self.new_archetype(info.source())?;
                        archetype.check_requirements()?;
                        let _ = archetype.render(render_context.extend_with(&info))?;
                    }
                    ArchetectAction::Connect { info, .. } => {
                        crate::client::start(render_context.extend_with(info), info.endpoint.to_string())?;
                    }
                }
                return Ok(());
            }
        }
    }
}
