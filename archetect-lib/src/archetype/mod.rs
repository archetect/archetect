use std::fs;
use std::path::{Path, PathBuf};

use linked_hash_map::LinkedHashMap;
use read_input::prelude::*;
use semver::{Version, VersionReq};

use crate::{Archetect, ArchetectError};
use crate::actions::ActionId;
use crate::config::{
    AnswerInfo, ArchetypeConfig, ArchetypeInfo, ModuleInfo, TemplateInfo,
};
use crate::errors::RenderError;
use crate::template_engine::{Context, Tera};
use crate::util::{Source, SourceError};
use crate::rules::RulesContext;

pub struct Archetype {
    tera: Tera,
    source: Source,
    config: ArchetypeConfig,
    path: PathBuf,
    modules: Vec<Module>,
}

impl Archetype {
    pub fn from_source(archetect: &Archetect, source: Source, offline: bool) -> Result<Archetype, ArchetypeError> {
        let tera = Tera::default();

        let local_path = source.local_path();

        let config = ArchetypeConfig::load(local_path)?;

        let mut archetype = Archetype {
            tera,
            config,
            source: source.clone(),
            path: local_path.to_owned(),
            modules: vec![],
        };

        let mut modules = vec![];

        for module_info in archetype.configuration().modules() {
            match module_info {
                ModuleInfo::Archetype(archetype_info) => {
                    let source = Source::detect(archetect, archetype_info.source(), Some(source.clone()))?;
                    let archetype = Archetype::from_source(archetect, source, offline)?;
                    modules.push(Module::Archetype(archetype, archetype_info.clone()));
                }
                ModuleInfo::TemplateDirectory(template_info) => {
                    modules.push(Module::Template(template_info.clone()));
                }
            }
        }

        for module in modules {
            archetype.modules.push(module);
        }

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

    fn render_string(&self, contents: &str, context: Context) -> Result<String, RenderError> {
        match self.tera.render_string(contents, context) {
            Ok(contents) => Ok(contents),
            Err(error) => Err(RenderError::StringRenderError {
                source: contents.to_owned(),
                error,
                message: "".to_string(),
            }),
        }
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

    pub fn get_context(
        &self,
        answers: &LinkedHashMap<String, AnswerInfo>,
        seed: Option<Context>,
    ) -> Result<Context, ArchetypeError> {
        let mut context = seed.unwrap_or_else(|| Context::new());

        if let Some(variables) = self.config.variables() {
            for (identifier, variable_info) in variables {
                // First, if an explicit answer was provided, use that, overriding an existing context
                // value if necessary.
                if let Some(answer) = answers.get(identifier) {
                    if let Some(value) = answer.value() {
                        context.insert(
                            identifier,
                            self.tera.render_string(value, context.clone()).unwrap().as_str(),
                        );
                    }
                }

                // If the context already contains a value, it was either inherited or answered, and
                // should therefore not be overwritten
                if context.contains(identifier) {
                    continue;
                }

                // Insert a value if one was specified in the archetype's configuration file.
                if let Some(value) = variable_info.value() {
                    context.insert(
                        identifier.as_str(),
                        self.tera.render_string(value, context.clone()).unwrap().as_str(),
                    );
                    continue;
                }

                // If we've reached this point, we'll need to prompt the user for an answer.

                // Determine if a default can be provided.
                let default = if let Some(answer) = answers.get(identifier) {
                    if let Some(default) = answer.default() {
                        Some(self.render_string(default, context.clone())?)
                    } else {
                        None
                    }
                } else if let Some(default) = variable_info.default() {
                    Some(self.render_string(default, context.clone())?)
                } else {
                    None
                };

                let mut prompt = if let Some(prompt) = variable_info.prompt() {
                    format!("{} ", prompt.trim())
                } else {
                    format!("{}: ", identifier)
                };

                if let Some(default) = &default {
                    prompt.push_str(format!("[{}] ", default).as_str());
                };

                let input_builder = input::<String>()
                    .msg(&prompt)
                    .add_test(|value| value.len() > 0)
                    .repeat_msg(&prompt)
                    .err("Must be at least 1 character.  Please try again.");
                let value = if let Some(default) = &default {
                    input_builder.default(default.clone().to_owned()).get()
                } else {
                    input_builder.get()
                };

                context.insert(identifier, &value);
            }
        }

        Ok(context)
    }
}

pub enum Module {
    Archetype(Archetype, ArchetypeInfo),
    Template(TemplateInfo),
}

impl Module {
    pub fn archetype(archetype: Archetype, archetype_info: ArchetypeInfo) -> Module {
        Module::Archetype(archetype, archetype_info)
    }

    pub fn template(template_info: TemplateInfo) -> Module {
        Module::Template(template_info)
    }
}

#[derive(Debug)]
pub enum ArchetypeError {
    ArchetypeInvalid,
    UnsatisfiedRequirements(Version, VersionReq),
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
