use std::sync::{Arc, Mutex};

use camino::{Utf8Path, Utf8PathBuf};
use semver::Version;

use archetect_api::{Artifact, ClientMessage, IoError, ScriptIoHandle, ScriptMessage};
use archetect_terminal_io::TerminalScriptIoHandle;

use crate::archive::ArchiveEntry;

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
    journal: Mutex<RenderJournal>,
    /// Capabilities this session grants. Unset means unrestricted — the local
    /// CLI, where the user *is* the trust boundary. Once set, anything not
    /// named is denied. `OnceLock` because a session's grants are established
    /// at initialization and must never widen afterwards.
    capabilities: std::sync::OnceLock<std::collections::HashSet<String>>,
}

/// What this render has produced so far.
///
/// `WriteFile` is a script→client message, so the rendered tree may never exist
/// on this side of the wire. The journal is therefore the only record we have —
/// archiving reads it back, and completion reports from it.
#[derive(Debug, Default)]
struct RenderJournal {
    files: Vec<(Utf8PathBuf, Vec<u8>)>,
    artifacts: Vec<Artifact>,
}

pub struct ArchetectBuilder {
    configuration: Option<Configuration>,
    layout: Option<Box<dyn SystemLayout>>,
    driver: Option<Box<dyn ScriptIoHandle>>,
    capabilities: Option<std::collections::HashSet<String>>,
}

impl ArchetectBuilder {
    /// Restrict this session to the given capabilities. Not calling this leaves
    /// the session unrestricted, which is correct for the local CLI and wrong
    /// for anything accepting renders from elsewhere.
    pub fn with_capabilities<I: IntoIterator<Item = String>>(mut self, capabilities: I) -> Self {
        self.capabilities = Some(capabilities.into_iter().collect());
        self
    }

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
        let archetect = Archetect::new(configuration, driver, layout);
        if let Some(capabilities) = self.capabilities {
            archetect.restrict_capabilities(capabilities);
        }
        Ok(archetect)
    }
}

impl Default for ArchetectBuilder {
    fn default() -> Self {
        ArchetectBuilder {
            configuration: None,
            layout: None,
            driver: None,
            capabilities: None,
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
                journal: Mutex::new(RenderJournal::default()),
                capabilities: std::sync::OnceLock::new(),
            }),
        }
    }

    /// Whether this session grants `capability`. An unrestricted session (the
    /// local CLI) grants everything; a session that enumerated its grants
    /// denies anything it did not name.
    pub fn grants(&self, capability: &str) -> bool {
        self.inner
            .capabilities
            .get()
            .is_none_or(|granted| granted.contains(capability))
    }

    /// Restrict this session to `capabilities`. Idempotent-by-construction: a
    /// second call is ignored, so a session cannot widen its own grants after
    /// initialization.
    pub fn restrict_capabilities<I: IntoIterator<Item = String>>(&self, capabilities: I) {
        let _ = self
            .inner
            .capabilities
            .set(capabilities.into_iter().collect());
    }

    pub fn builder() -> ArchetectBuilder {
        ArchetectBuilder::default()
    }

    /// Reap materialized source trees unused longer than the configured retention (skipping any a
    /// session still holds). Returns `(removed, kept, in_use)`.
    pub fn prune_cache(&self) -> Result<(usize, usize, usize), crate::errors::SourceError> {
        let retention = self
            .configuration()
            .updates()
            .retention()
            .to_std()
            .unwrap_or_else(|_| std::time::Duration::from_secs(7_776_000));
        let stats = archetect_git_cache::prune(&self.layout().cache_dir(), retention)?;
        Ok((stats.removed, stats.kept, stats.in_use))
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
        if let ScriptMessage::WriteFile(info) = &command {
            if let Ok(mut journal) = self.inner.journal.lock() {
                journal
                    .files
                    .push((Utf8PathBuf::from(&info.destination), info.contents.clone()));
            }
        }
        self.inner.io_driver.send(command)
    }

    /// Every file this render wrote beneath `dir`, addressed relative to it.
    ///
    /// Note this is what the render *produced*, not what happens to be on disk:
    /// files a shell-out created are invisible here, because they never crossed
    /// the IO channel.
    pub fn archive_entries_under(&self, dir: &Utf8Path) -> Vec<ArchiveEntry> {
        let Ok(journal) = self.inner.journal.lock() else {
            return Vec::new();
        };
        journal
            .files
            .iter()
            .filter_map(|(path, contents)| {
                let relative = path.strip_prefix(dir).ok()?;
                Some(ArchiveEntry {
                    path: relative.as_str().replace('\\', "/"),
                    contents: contents.clone(),
                })
            })
            .collect()
    }

    pub fn record_artifact(&self, artifact: Artifact) {
        if let Ok(mut journal) = self.inner.journal.lock() {
            journal.artifacts.push(artifact);
        }
    }

    pub fn artifacts(&self) -> Vec<Artifact> {
        self.inner
            .journal
            .lock()
            .map(|journal| journal.artifacts.clone())
            .unwrap_or_default()
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
