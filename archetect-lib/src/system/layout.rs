use crate::system::SystemError;
use directories::ProjectDirs;
use std::fmt::{Display, Error, Formatter};
use std::path::{Path, PathBuf};
use tempfile::tempdir;

pub enum LayoutType {
    Native,
    DotHome,
    Temp,
}

pub trait SystemLayout {
    fn configs_dir(&self) -> PathBuf;

    fn cache_dir(&self) -> PathBuf;

    fn catalog_cache_dir(&self) -> PathBuf {
        self.cache_dir().join("catalogs")
    }

    fn git_cache_dir(&self) -> PathBuf {
        self.cache_dir().join("git")
    }

    fn http_cache_dir(&self) -> PathBuf {
        self.cache_dir().join("http")
    }

    fn answers_config(&self) -> PathBuf {
        self.configs_dir().join("answers.yaml")
    }

    fn catalog(&self) -> PathBuf {
        self.configs_dir().join("catalog.toml")
    }

    fn catalog_registry(&self) -> PathBuf {
        self.configs_dir().join("catalog_registry.toml")
    }

    fn user_config(&self) -> PathBuf {
        self.configs_dir().join("archetect.toml")
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
    fn configs_dir(&self) -> PathBuf {
        self.project.config_dir().to_owned()
    }

    fn cache_dir(&self) -> PathBuf {
        self.project.cache_dir().to_owned()
    }
}

#[derive(Debug)]
pub struct RootedSystemLayout {
    directory: PathBuf,
}

impl RootedSystemLayout {
    pub fn new<D: AsRef<Path>>(directory: D) -> Result<RootedSystemLayout, SystemError> {
        let directory = directory.as_ref();
        let directory = directory.to_owned();
        Ok(RootedSystemLayout { directory })
    }
}

impl SystemLayout for RootedSystemLayout {
    fn configs_dir(&self) -> PathBuf {
        self.directory.clone().join("etc")
    }

    fn cache_dir(&self) -> PathBuf {
        self.directory.clone().join("var")
    }
}

impl Display for dyn SystemLayout {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        writeln!(f, "{}: {}", "Configs Directory", self.configs_dir().display())?;
        writeln!(f, "{}: {}", "User Answers", self.answers_config().display())?;
        writeln!(f, "{}: {}", "User Catalog", self.catalog().display())?;
        writeln!(f, "{}: {}", "User Config", self.user_config().display())?;
        writeln!(f, "{}: {}", "Git Cache", self.git_cache_dir().display())?;
        writeln!(f, "{}: {}", "Catalog Cache", self.catalog_cache_dir().display())?;
        Ok(())
    }
}

pub fn dot_home_layout() -> Result<RootedSystemLayout, SystemError> {
    let result = shellexpand::full("~/.archetect/").unwrap();
    Ok(RootedSystemLayout::new(result.to_string())?)
}

pub fn temp_layout() -> Result<RootedSystemLayout, SystemError> {
    let temp_dir = tempdir()?;
    Ok(RootedSystemLayout::new(temp_dir.path())?)
}

#[cfg(test)]
mod tests {
    use crate::system::layout::{NativeSystemLayout, RootedSystemLayout, SystemLayout};

    #[test]
    fn test_native_system_paths() {
        let native_paths: Box<dyn SystemLayout> = Box::new(NativeSystemLayout::new().unwrap());
        print!("{}", native_paths);
    }

    #[test]
    fn test_directory_system_paths() {
        let native_paths: Box<dyn SystemLayout> = Box::new(RootedSystemLayout::new("~/.archetect/").unwrap());
        print!("{}", native_paths);
    }
}
