use directories::ProjectDirs;
use std::path::PathBuf;

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
