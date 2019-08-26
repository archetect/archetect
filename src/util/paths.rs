use directories::ProjectDirs;
use std::path::{PathBuf, Path};
use std::fmt::{Display, Formatter, Error};

pub fn project_dirs() -> ProjectDirs {
    ProjectDirs::from("", "", "archetect").unwrap()
}

pub fn configs_dir() -> PathBuf {
    project_dirs().config_dir().to_owned()
}

pub fn answers_config() -> PathBuf {
    configs_dir().join("answers.toml")
}

pub fn cache_dir() -> PathBuf {
    project_dirs().cache_dir().to_owned()
}

pub fn git_cache_dir() -> PathBuf {
    cache_dir().join("git")
}

pub fn catalog_cache_dir() -> PathBuf {
    cache_dir().join("catalogs")
}


pub trait SystemPaths {
    fn configs_dir(&self) -> PathBuf;

    fn cache_dir(&self) -> PathBuf;

    fn catalog_cache_dir(&self) -> PathBuf {
        self.cache_dir().join("catalogs")
    }

    fn git_cache_dir(&self) -> PathBuf {
        self.cache_dir().join("git")
    }

    fn answers_config(&self) -> PathBuf {
        self.configs_dir().join("answers.toml")
    }

    fn user_config(&self) -> PathBuf {
        self.configs_dir().join("archetect.toml")
    }
}

#[derive(Debug)]
pub struct NativeSystemPaths {
    project: ProjectDirs,
}

impl NativeSystemPaths {
    pub fn new() -> Result<NativeSystemPaths, String> {
        match ProjectDirs::from("", "", "archetect") {
            Some(project) => Ok(NativeSystemPaths { project }),
            None => Err("No home directory detected for the current user.".to_owned()),
        }
    }
}

impl SystemPaths for NativeSystemPaths {
    fn configs_dir(&self) -> PathBuf {
        self.project.config_dir().to_owned()
    }

    fn cache_dir(&self) -> PathBuf {
        self.project.cache_dir().to_owned()
    }
}

#[derive(Debug)]
pub struct DirectorySystemPaths {
    directory: PathBuf,
}

impl DirectorySystemPaths {
    pub fn new<D: AsRef<Path>>(directory: D) -> Result<DirectorySystemPaths, String> {
        let directory = directory.as_ref();
        let directory = directory.to_owned();
        Ok(DirectorySystemPaths { directory })
    }
}

impl SystemPaths for DirectorySystemPaths {
    fn configs_dir(&self) -> PathBuf {
        self.directory.clone().join("etc")
    }

    fn cache_dir(&self) -> PathBuf {
        self.directory.clone().join("var")
    }
}

impl Display for dyn SystemPaths {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        writeln!(f, "{}: {}", "Configs Directory", self.configs_dir().display())?;
        writeln!(f, "{}: {}", "User Answers", self.answers_config().display())?;
        writeln!(f, "{}: {}", "User Config", self.user_config().display())?;
        writeln!(f, "{}: {}", "Git Cache", self.git_cache_dir().display())?;
        writeln!(f, "{}: {}", "Catalog Cache", self.catalog_cache_dir().display())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::util::paths::{NativeSystemPaths, SystemPaths, DirectorySystemPaths};

    #[test]
    fn test_native_system_paths() {
        let native_paths: Box<dyn SystemPaths> = Box::new(NativeSystemPaths::new().unwrap());
        print!("{}", native_paths);
    }

    #[test]
    fn test_directory_system_paths() {
        let native_paths: Box<dyn SystemPaths> = Box::new(DirectorySystemPaths::new("~/.archetect/").unwrap());
        print!("{}", native_paths);
    }
}