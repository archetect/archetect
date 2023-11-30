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
    fn configs_dir(&self) -> Utf8PathBuf;

    fn cache_dir(&self) -> Utf8PathBuf;

    fn catalog_cache_dir(&self) -> Utf8PathBuf {
        self.cache_dir().join("catalogs")
    }

    fn git_cache_dir(&self) -> Utf8PathBuf {
        self.cache_dir().join("git")
    }

    fn http_cache_dir(&self) -> Utf8PathBuf {
        self.cache_dir().join("http")
    }

    fn answers_config(&self) -> Utf8PathBuf {
        self.configs_dir().join("answers.yml")
    }

    fn configuration_path(&self) -> Utf8PathBuf {
        self.configs_dir().join("archetect.yaml")
    }

    fn catalog(&self) -> Utf8PathBuf {
        self.configs_dir().join("catalog.yml")
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
    fn configs_dir(&self) -> Utf8PathBuf {
        Utf8PathBuf::from_path_buf(self.project.config_dir().to_owned()).unwrap()
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

        if !layout.answers_config().exists() {
            if layout.configs_dir().join("answers.yaml").exists() {
                std::fs::rename(layout.configs_dir().join("answers.yaml"), layout.answers_config())?;
            }
        }

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
    fn configs_dir(&self) -> Utf8PathBuf {
        self.directory.clone().join("etc")
    }

    fn cache_dir(&self) -> Utf8PathBuf {
        self.directory.clone().join("var")
    }
}

impl Display for dyn SystemLayout {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        writeln!(f, "{}: {}", "Configs Directory", self.configs_dir())?;
        writeln!(f, "{}: {}", "User Answers", self.answers_config())?;
        writeln!(f, "{}: {}", "Git Cache", self.git_cache_dir())?;
        writeln!(f, "{}: {}", "Catalog Cache", self.catalog_cache_dir())?;
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
