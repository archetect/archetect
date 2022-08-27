use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use linked_hash_map::LinkedHashMap;

use crate::actions::ActionId;
use crate::config::{AnswerInfo, ArchetypeConfig};
use crate::errors::RenderError;
use crate::rules::RulesContext;
use crate::vendor::tera::Context;
use crate::source::{Source, SourceError};
use crate::{Archetect, ArchetectError};
use crate::scripting::lua::LuaScriptContext;
use crate::scripting::rhai::RhaiScriptContext;

#[derive(Clone)]
pub struct Archetype {
    source: Source,
    config: ArchetypeConfig,
}

impl Archetype {
    pub fn from_source(source: &Source) -> Result<Archetype, ArchetypeError> {
        let config = ArchetypeConfig::load(source.local_path())?;

        let archetype = Archetype {
            config,
            source: source.clone(),
        };

        Ok(archetype)
    }

    pub fn configuration(&self) -> &ArchetypeConfig {
        &self.config
    }

    pub fn source(&self) -> &Source {
        &self.source
    }

    pub fn render<D: AsRef<Path>>(
        &self,
        archetect: &mut Archetect,
        destination: D,
        answers: &LinkedHashMap<String, AnswerInfo>,
    ) -> Result<(), ArchetectError> {
        let destination = destination.as_ref();
        fs::create_dir_all(destination)?;

        let mut rules_context = RulesContext::new();
        let mut context = Context::new();

        let archetect_info = ArchetectInfo {
            offline: archetect.offline(),
            version: clap::crate_version!().to_owned(),
        };
        context.insert("archetect", &archetect_info);

        let archetype_info = ArchetypeInfo {
            source: self.source().source().to_owned(),
            destination: destination.to_str().unwrap().to_owned(),
            local_path: self.source().local_path().to_str().unwrap().to_owned(),
        };
        context.insert("archetype", &archetype_info);

        let root_action = ActionId::from(self.config.actions());

        root_action.execute(archetect, self, destination, &mut rules_context, answers, &mut context)?;

        if let Some(script) = self.configuration().script() {
            if script.ends_with(".lua") {
                let archetect = Archetect::build()?;
                let script_context = LuaScriptContext::new(Arc::new(Mutex::new(archetect)), Arc::new(Mutex::new(self
                    .clone())));
                script_context.execute_archetype(&self)?
            } else if script.ends_with(".rhai") {
                let script_context = RhaiScriptContext::new();
                script_context.execute(&self)?
            } else {
                return Err(ArchetectError::ArchetypeError(ArchetypeError::UnsupportedScriptType(script.to_string())));
            }
        }

        Ok(())
    }
}

// TODO: Rework to capture working directory
#[derive(Debug, Serialize, Deserialize)]
pub struct ArchetypeInfo {
    source: String,
    destination: String,
    local_path: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ArchetectInfo {
    offline: bool,
    version: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ArchetypeError {
    #[error("The specified archetype is missing an archetype.yml or archetype.yaml file")]
    ArchetypeConfigMissing,
    #[error("The specified archetype config `{path}` does not exist")]
    ArchetypeConfigNotFound {
        path: PathBuf,
    },
    #[error("Invalid Answers Config")]
    InvalidAnswersConfig,
    #[error(transparent)]
    SourceError(#[from] SourceError),
    #[error(transparent)]
    RenderError(#[from] RenderError),
    #[error("IO Error in Archetype: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Unsupported script type: {0}")]
    UnsupportedScriptType(String),
    #[error("Archetype Configuration Error in `{path}`: {source}")]
    YamlError {
        path: PathBuf,
        source: serde_yaml::Error
    },
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
