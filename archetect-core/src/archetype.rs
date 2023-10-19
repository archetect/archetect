use camino::Utf8PathBuf;
use std::path::PathBuf;

use rhai::EvalAltResult;

use crate::errors::RenderError;
use crate::requirements::RequirementsError;
#[derive(Debug, thiserror::Error)]
pub enum ArchetypeError {
    #[error("The specified archetype is missing an archetype.yml or archetype.yaml file")]
    ArchetypeConfigMissing,
    #[error("The specified archetype config `{path}` does not exist")]
    ArchetypeConfigNotFound { path: PathBuf },
    #[error("The specified archetype manifest `{path}` does not exist")]
    ArchetypeManifestNotFound { path: Utf8PathBuf },
    #[error("Archetype by key `{key}` does not exist")]
    ArchetypeKeyNotFound { key: String },
    #[error(transparent)]
    ArchetypeScriptError(#[from] EvalAltResult),
    #[error("Invalid Answers Config")]
    InvalidAnswersConfig,
    #[error(transparent)]
    RenderError(#[from] RenderError),
    #[error("IO Error in Archetype: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Archetype Configuration Error in `{path}`: {source}")]
    YamlError { path: PathBuf, source: serde_yaml::Error },
    #[error("Archetype Manifest Syntax error in `{path}`: {source}")]
    ArchetypeManifestSyntaxError {
        path: Utf8PathBuf,
        source: serde_yaml::Error,
    },
    #[error("Operation was interrupted")]
    OperationInterrupted,
    #[error("Value is required")]
    ValueRequired,
    #[error("Archetype requirements failure:\n\n{0}")]
    RequirementsError(#[from] RequirementsError),
}

#[cfg(test)]
mod tests {
    use glob::Pattern;
    use std::path::Path;

    #[test]
    fn test_glob_full_directory_path() {
        assert!(Pattern::new("*/projects")
            .unwrap()
            .matches_path(Path::new("/home/luser/projects")));
        assert!(Pattern::new("/home/*/projects")
            .unwrap()
            .matches_path(Path::new("/home/luser/projects")));
        assert!(Pattern::new("/home/*/projects*")
            .unwrap()
            .matches_path(Path::new("/home/luser/projects")));
        assert!(Pattern::new("/h*/*/*ects")
            .unwrap()
            .matches_path(Path::new("/home/luser/projects")));
        assert!(Pattern::new("*/{{ name # train_case }}/*")
            .unwrap()
            .matches_path(Path::new("/home/{{ name # train_case }}/projects")));
        assert!(Pattern::new("*/{{ name | train_case }}/*")
            .unwrap()
            .matches_path(Path::new("/home/{{ name | train_case }}/projects")));
    }

    #[test]
    fn test_glob_full_file_path() {
        assert!(Pattern::new("*/projects/*")
            .unwrap()
            .matches_path(Path::new("/home/luser/projects/image.jpg")));
        assert!(Pattern::new("/home/*/projects/*")
            .unwrap()
            .matches_path(Path::new("/home/luser/projects/image.jpg")));
        assert!(Pattern::new("/h*/*/*ects*jpg")
            .unwrap()
            .matches_path(Path::new("/home/luser/projects/image.jpg")));
        assert!(Pattern::new("*.jpg")
            .unwrap()
            .matches_path(Path::new("/home/luser/projects/image.jpg")));
        assert!(Pattern::new("/home/**/*.jpg")
            .unwrap()
            .matches_path(Path::new("/home/luser/projects/image.jpg")));
        assert!(Pattern::new("*/{{ name # train_case }}/*")
            .unwrap()
            .matches_path(Path::new("/home/{{ name # train_case }}/projects")));
        assert!(Pattern::new("*/{{ name | train_case }}/*")
            .unwrap()
            .matches_path(Path::new("/home/{{ name | train_case }}/projects")));
    }
}
