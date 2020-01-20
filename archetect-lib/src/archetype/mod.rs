use std::fs;
use std::path::{Path, PathBuf};

use linked_hash_map::LinkedHashMap;

use crate::{Archetect, ArchetectError};
use crate::actions::ActionId;
use crate::config::{
    AnswerInfo, ArchetypeConfig,
};
use crate::errors::RenderError;
use crate::rules::RulesContext;
use crate::template_engine::Context;
use crate::util::{Source, SourceError};

pub struct Archetype {
    source: Source,
    config: ArchetypeConfig,
    path: PathBuf,
}

impl Archetype {
    pub fn from_source(source: &Source) -> Result<Archetype, ArchetypeError> {
        let local_path = source.local_path();

        let config = ArchetypeConfig::load(local_path)?;

        let archetype = Archetype {
            config,
            source: source.clone(),
            path: local_path.to_owned(),
        };

        Ok(archetype)
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    pub fn configuration(&self) -> &ArchetypeConfig {
        &self.config
    }

    pub fn source(&self) -> &Source {
        &self.source
    }

    pub fn execute_script<D: AsRef<Path>>(&self,
                                          archetect: &Archetect,
                                          destination: D,
                                          answers: &LinkedHashMap<String, AnswerInfo>,
    ) -> Result<(), ArchetectError> {
        let destination = destination.as_ref();
        fs::create_dir_all(destination).unwrap();

        let mut rules_context = RulesContext::new();
        let mut context = Context::new();

        let root_action = ActionId::from(self.config.actions());

        root_action.execute(archetect, self, destination, &mut rules_context, answers, &mut context)
    }
}

#[derive(Debug)]
pub enum ArchetypeError {
    ArchetypeInvalid,
    InvalidAnswersConfig,
    ArchetypeSaveFailed,
    SourceError(SourceError),
    RenderError(RenderError),
}

impl From<SourceError> for ArchetypeError {
    fn from(cause: SourceError) -> Self {
        ArchetypeError::SourceError(cause)
    }
}

impl From<RenderError> for ArchetypeError {
    fn from(error: RenderError) -> Self {
        ArchetypeError::RenderError(error)
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use glob::Pattern;

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
