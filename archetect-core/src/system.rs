use crate::errors::SystemError;
use crate::utils::to_utf8_path;
use camino::{Utf8Path, Utf8PathBuf};
use directories::ProjectDirs;
use std::fmt::{Debug, Display, Error, Formatter};
use tempfile::tempdir;

pub enum LayoutType {
    Native,
    DotHome,
    Temp,
}

pub trait SystemLayout: Debug + Send + Sync + 'static {
    fn etc_dir(&self) -> Utf8PathBuf;

    fn etc_d_dir(&self) -> Utf8PathBuf;

    fn cache_dir(&self) -> Utf8PathBuf;

    fn configuration_path(&self) -> Utf8PathBuf {
        self.etc_dir().join("archetect.yaml")
    }
}

impl<T: SystemLayout> From<T> for Box<dyn SystemLayout> {
    fn from(value: T) -> Self {
        Box::new(value)
    }
}

#[derive(Debug)]
pub struct NativeSystemLayout {
    project: ProjectDirs,
}

impl NativeSystemLayout {
    pub fn new() -> Result<NativeSystemLayout, SystemError> {
        match ProjectDirs::from("", "", "archetect") {
            Some(project) => Ok(NativeSystemLayout { project }),
            None => Err(SystemError::GenericError(
                "No home directory detected for the current user.".to_owned(),
            )),
        }
    }
}

impl SystemLayout for NativeSystemLayout {
    fn etc_dir(&self) -> Utf8PathBuf {
        Utf8PathBuf::from_path_buf(self.project.config_dir().to_owned()).unwrap()
    }

    fn etc_d_dir(&self) -> Utf8PathBuf {
        // For NativeSystemLayout, etc.d is at ~/.archetect/etc.d
        Utf8PathBuf::from_path_buf(self.project.config_dir().to_owned()).unwrap().join("etc.d")
    }

    fn cache_dir(&self) -> Utf8PathBuf {
        Utf8PathBuf::from_path_buf(self.project.cache_dir().to_owned()).unwrap()
    }
}

#[derive(Debug)]
pub struct RootedSystemLayout {
    directory: Utf8PathBuf,
}

impl RootedSystemLayout {
    pub fn new<D: AsRef<Utf8Path>>(directory: D) -> Result<RootedSystemLayout, SystemError> {
        let directory = directory.as_ref();
        let directory = directory.to_owned();
        let layout = RootedSystemLayout { directory };
        Ok(layout)
    }

    pub fn temp() -> Result<RootedSystemLayout, SystemError> {
        temp_layout()
    }

    pub fn dot_home() -> Result<RootedSystemLayout, SystemError> {
        dot_home_layout()
    }
}

impl SystemLayout for RootedSystemLayout {
    fn etc_dir(&self) -> Utf8PathBuf {
        self.directory.clone().join("etc")
    }

    fn etc_d_dir(&self) -> Utf8PathBuf {
        // For RootedSystemLayout (used in tests), etc.d is at {root}/etc.d
        self.directory.clone().join("etc.d")
    }

    fn cache_dir(&self) -> Utf8PathBuf {
        self.directory.clone().join("cache")
    }
}

impl Display for dyn SystemLayout {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        writeln!(f, "{}: {}", "Etc Directory", self.etc_dir())?;
        writeln!(f, "{}: {}", "Etc.d Directory", self.etc_d_dir())?;
        writeln!(f, "{}: {}", "Cache Directory", self.cache_dir())?;
        Ok(())
    }
}

pub fn dot_home_layout() -> Result<RootedSystemLayout, SystemError> {
    let result = directories::UserDirs::new().unwrap().home_dir().join(".archetect");
    Ok(RootedSystemLayout::new(result.to_str().unwrap().to_string())?)
}

pub fn temp_layout() -> Result<RootedSystemLayout, SystemError> {
    let temp_dir = tempdir()?;
    Ok(RootedSystemLayout::new(to_utf8_path(temp_dir.path()))?)
}
