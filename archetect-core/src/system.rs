use std::fmt::{Debug, Display, Error, Formatter};

use camino::{Utf8Path, Utf8PathBuf};
use tempfile::tempdir;

use crate::errors::SystemError;
use crate::utils::to_utf8_path;

const APP_NAME: &str = "archetect";

pub trait SystemLayout: Debug + Send + Sync + 'static {
    /// Configuration directory. Holds `archetect.yaml`.
    fn etc_dir(&self) -> Utf8PathBuf;

    /// Drop-in configuration directory for additional YAML fragments.
    fn etc_d_dir(&self) -> Utf8PathBuf;

    /// Cache directory for git clones, HTTP downloads, and other regenerable data.
    fn cache_dir(&self) -> Utf8PathBuf;

    /// Persistent data directory for things like Lua type annotations.
    /// This is data that should survive cache wipes but is owned by Archetect.
    fn data_dir(&self) -> Utf8PathBuf;

    /// Convenience: path to the main configuration file.
    fn configuration_path(&self) -> Utf8PathBuf {
        self.etc_dir().join("archetect.yaml")
    }
}

impl<T: SystemLayout> From<T> for Box<dyn SystemLayout> {
    fn from(value: T) -> Self {
        Box::new(value)
    }
}

/// XDG-style system layout for production use.
///
/// On Linux and macOS, paths follow the XDG Base Directory Specification:
///   - config: `$XDG_CONFIG_HOME/archetect` (default `~/.config/archetect`)
///   - cache:  `$XDG_CACHE_HOME/archetect`  (default `~/.cache/archetect`)
///   - data:   `$XDG_DATA_HOME/archetect`   (default `~/.local/share/archetect`)
///
/// On Windows, native conventions are used:
///   - config: `%APPDATA%\archetect\config`
///   - cache:  `%LOCALAPPDATA%\archetect\cache`
///   - data:   `%APPDATA%\archetect\data`
#[derive(Debug)]
pub struct XdgSystemLayout {
    config_dir: Utf8PathBuf,
    cache_dir: Utf8PathBuf,
    data_dir: Utf8PathBuf,
}

impl XdgSystemLayout {
    pub fn new() -> Result<XdgSystemLayout, SystemError> {
        #[cfg(unix)]
        let (config_dir, cache_dir, data_dir) = unix_xdg_paths()?;

        #[cfg(windows)]
        let (config_dir, cache_dir, data_dir) = windows_paths()?;

        Ok(XdgSystemLayout {
            config_dir,
            cache_dir,
            data_dir,
        })
    }
}

#[cfg(unix)]
fn unix_xdg_paths() -> Result<(Utf8PathBuf, Utf8PathBuf, Utf8PathBuf), SystemError> {
    use etcetera::base_strategy::{BaseStrategy, Xdg};

    let xdg = Xdg::new().map_err(|e| {
        SystemError::HomeDirectoryNotFound(format!("XDG layout init failed: {}", e))
    })?;

    let config_dir = to_utf8(xdg.config_dir().join(APP_NAME))?;
    let cache_dir = to_utf8(xdg.cache_dir().join(APP_NAME))?;
    let data_dir = to_utf8(xdg.data_dir().join(APP_NAME))?;

    Ok((config_dir, cache_dir, data_dir))
}

#[cfg(windows)]
fn windows_paths() -> Result<(Utf8PathBuf, Utf8PathBuf, Utf8PathBuf), SystemError> {
    use etcetera::base_strategy::{BaseStrategy, Windows};

    let win = Windows::new().map_err(|e| {
        SystemError::HomeDirectoryNotFound(format!("Windows layout init failed: {}", e))
    })?;

    // Native Windows convention: AppData/archetect/{config,cache,data}
    let config_dir = to_utf8(win.config_dir().join(APP_NAME).join("config"))?;
    let cache_dir = to_utf8(win.cache_dir().join(APP_NAME).join("cache"))?;
    let data_dir = to_utf8(win.data_dir().join(APP_NAME).join("data"))?;

    Ok((config_dir, cache_dir, data_dir))
}

fn to_utf8(path: std::path::PathBuf) -> Result<Utf8PathBuf, SystemError> {
    Utf8PathBuf::from_path_buf(path).map_err(|p| {
        SystemError::HomeDirectoryNotFound(format!("Non-UTF-8 path: {}", p.display()))
    })
}

impl SystemLayout for XdgSystemLayout {
    fn etc_dir(&self) -> Utf8PathBuf {
        self.config_dir.clone()
    }

    fn etc_d_dir(&self) -> Utf8PathBuf {
        self.config_dir.join("etc.d")
    }

    fn cache_dir(&self) -> Utf8PathBuf {
        self.cache_dir.clone()
    }

    fn data_dir(&self) -> Utf8PathBuf {
        self.data_dir.clone()
    }
}

/// Layout rooted at a user-specified directory. Used by tests and integration
/// scenarios that need a fully isolated environment. The structure under the
/// root mirrors XDG semantics:
///
/// ```text
/// <root>/
///   etc/        ← config
///   etc.d/      ← drop-in config fragments
///   cache/      ← regenerable downloads
///   data/       ← persistent data (annotations, etc.)
/// ```
#[derive(Debug)]
pub struct RootedSystemLayout {
    directory: Utf8PathBuf,
}

impl RootedSystemLayout {
    pub fn new<D: AsRef<Utf8Path>>(directory: D) -> Result<RootedSystemLayout, SystemError> {
        Ok(RootedSystemLayout {
            directory: directory.as_ref().to_owned(),
        })
    }

    /// Build an isolated layout in a fresh temp directory. Used by integration tests.
    pub fn temp() -> Result<RootedSystemLayout, SystemError> {
        let temp_dir = tempdir()?;
        Self::new(to_utf8_path(temp_dir.path()))
    }
}

impl SystemLayout for RootedSystemLayout {
    fn etc_dir(&self) -> Utf8PathBuf {
        self.directory.join("etc")
    }

    fn etc_d_dir(&self) -> Utf8PathBuf {
        self.directory.join("etc.d")
    }

    fn cache_dir(&self) -> Utf8PathBuf {
        self.directory.join("cache")
    }

    fn data_dir(&self) -> Utf8PathBuf {
        self.directory.join("data")
    }
}

impl Display for dyn SystemLayout {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        writeln!(f, "Etc Directory:   {}", self.etc_dir())?;
        writeln!(f, "Etc.d Directory: {}", self.etc_d_dir())?;
        writeln!(f, "Cache Directory: {}", self.cache_dir())?;
        writeln!(f, "Data Directory:  {}", self.data_dir())?;
        Ok(())
    }
}
