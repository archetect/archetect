#[macro_use]
extern crate pest_derive;
#[macro_use]
extern crate serde_derive;
#[cfg_attr(test, macro_use)]
extern crate serde_json;
#[macro_use]
extern crate lazy_static;
#[cfg(test)]
#[macro_use]
extern crate pretty_assertions;

pub mod config;
pub mod errors;
pub mod heck;
pub mod input;
pub mod loggerv;
pub mod template_engine;
pub mod util;

use log::{trace, warn};
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::{PathBuf, Path};

use read_input::prelude::*;
use template_engine::{Context, Tera};

use crate::config::{ArchetypeConfig, PathRuleConfig};
use crate::config::ModuleConfig;
use crate::config::{Answer, PatternType};
use crate::util::{Source, SourceError};
use std::collections::HashMap;
use crate::errors::RenderError;

pub struct Archetype {
    tera: Tera,
    config: ArchetypeConfig,
    path: PathBuf,
    modules: Vec<Module>,
}

impl Archetype {
    pub fn from_source(source: Source, offline: bool) -> Result<Archetype, ArchetypeError> {
        let tera = Tera::default();

        let local_path = source.local_path();

        let config = ArchetypeConfig::load(local_path)?;

        let mut archetype = Archetype {
            tera,
            config,
            path: local_path.to_owned(),
            modules: vec![],
        };

        let mut modules = vec![];

        for module_config in archetype.configuration().modules() {
            let source = Source::detect(module_config.source(), offline, Some(source.clone()))?;
            let module_archetype = Archetype::from_source(source, offline)?;
            modules.push(Module::new(module_config.clone(), module_archetype));
        }

        for module in modules {
            archetype.modules.push(module);
        }


        Ok(archetype)
    }

    pub fn configuration(&self) -> &ArchetypeConfig {
        &self.config
    }

    fn render_directory<SRC: Into<PathBuf>, DEST: Into<PathBuf>>(
        &self,
        context: Context,
        source: SRC,
        destination: DEST,
    ) -> Result<(), RenderError> {
        let source = source.into();
        let destination = destination.into();

        if !source.is_dir() {
            if self.configuration().modules().is_empty() {
                warn!(
                    "The archetype's '{}' directory does not exist, and there are no submodules. Nothing to render.",
                    source.display()
                );
            }
            return Ok(());
        }

        'walking: for entry in fs::read_dir(&source)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                let destination = self.render_destination(&destination, &path, &context)?;
                trace!("Generating {:?}", &destination);
                fs::create_dir_all(destination.as_path()).unwrap();
                self.render_directory(context.clone(), path, destination)?;
            } else if path.is_file() {
                match self.match_rules(&path) {
                    Ok(None) => {
                        let destination = self.render_destination(&destination, &path, &context)?;
                        let contents = self.render_contents(&path, &context)?;
                        self.write_contents(&destination, &contents)?;
                    },
                    Ok(Some(rule)) => {
                        let destination = self.render_destination(&destination, &path, &context)?;
                        if rule.filter().unwrap_or(true) {
                            let contents = self.render_contents(&path, &context)?;
                            self.write_contents(&destination, &contents)?;
                        } else {
                            self.copy_contents(&path, &destination)?;
                        }
                    },
                    Err(err) => return Err(err),
                };
            }
        }

        Ok(())
    }

    fn match_rules<P: AsRef<Path>>(&self, path: P) -> Result<Option<PathRuleConfig>, RenderError> {
        let path= path.as_ref();
        for path_rule in self.configuration().path_rules() {
            if path_rule.pattern_type() == &PatternType::GLOB {
                for pattern in path_rule.patterns() {
                    let matcher = glob::Pattern::new(pattern).unwrap();
                    if matcher.matches_path(&path) {
                        return Ok(Some(path_rule.to_owned()));
                    }
                }
            }
        }
        Ok(None)
    }

    fn render_path<P: AsRef<Path>>(&self, path: P, context: &Context) -> Result<String, RenderError> {
        let path = path.as_ref();
        match self
            .tera
            .render_string(path.file_name().unwrap().to_str().unwrap(), context.clone()) {
            Ok(result) => Ok(result),
            Err(error) => {
                // TODO: Get a better error message.
                let message = String::new();
                Err(RenderError::PathRenderError { source: path.into(), error, message })
            }
        }
    }

    fn render_destination<P: AsRef<Path>, C: AsRef<Path>>(&self, parent: P, child: C, context: &Context) -> Result<PathBuf, RenderError> {
        let mut destination = parent.as_ref().to_owned();
        let child = child.as_ref();
        let name = self.render_path(&child, &context)?;
        destination.push(name);
        Ok(destination)
    }

    fn render_contents<P: AsRef<Path>>(&self, path: P, context: &Context) -> Result<String, RenderError> {
        let path = path.as_ref();
        let template = fs::read_to_string(path)?;
        match self.tera.render_string(&template, context.clone()) {
            Ok(result) => Ok(result),
            Err(error) => {
                // TODO: Get a better error message.
                let message = String::new();
                Err(RenderError::FileRenderError { source: path.into(), error, message })
            }
        }
    }

    fn write_contents<P: AsRef<Path>>(&self, destination: P, contents: &str) -> Result<(), RenderError> {
        let destination = destination.as_ref();
        trace!("Generating {:?}", destination);
        let mut output = File::create(&destination)?;
        output.write(contents.as_bytes())?;
        Ok(())
    }

    fn copy_contents<S: AsRef<Path>, D: AsRef<Path>>(&self, source: S, destination: D) -> Result<(), RenderError> {
        let source = source.as_ref();
        let destination = destination.as_ref();
        trace!("Copying    {:?}", destination);
        fs::copy(source, destination)?;
        Ok(())

    }

    pub fn render<D: Into<PathBuf>>(&self, destination: D, context: Context) -> Result<(), ArchetypeError> {
        let destination = destination.into();
        fs::create_dir_all(&destination).unwrap();
        self.render_directory(
            context.clone(),
            self.path.clone().join(self.configuration().contents_dir()),
            destination,
        )?;

        for module in &self.modules {
            let destination = PathBuf::from(
                self.tera
                    .render_string(module.config.destination(), context.clone())
                    .unwrap(),
            );
            let mut answers = HashMap::new();
            if let Some(answer_configs) = module.config.answers() {
                for answer in answer_configs {
                    answers.insert(
                        answer.identifier().to_owned(),
                        Answer::new(
                            answer.identifier(),
                            &self.tera.render_string(answer.value(), context.clone()).unwrap(),
                        ),
                    );
                }
            }
            let context = module.archetype.get_context(&answers)?;
            module.archetype.render(destination, context)?;
        }
        Ok(())
    }

    pub fn get_context(&self, answers: &HashMap<String, Answer>) -> Result<Context, ArchetypeError> {
        let mut context = Context::new();

        for variable in self.config.variables() {
            let default = if let Some(answer) = answers.get(variable.identifier()) {
                if let Some(true) = answer.prompt() {
                    Some(self.tera.render_string(answer.value(), context.clone()).unwrap())
                } else {
                    context.insert(
                        answer.identifier(),
                        self.tera
                            .render_string(answer.value(), context.clone())
                            .unwrap()
                            .as_str(),
                    );
                    continue;
                }
            } else if let Some(default) = variable.default().clone() {
                Some(self.tera.render_string(default, context.clone()).unwrap())
            } else {
                None
            };

            if let Some(prompt) = variable.prompt() {
                let prompt = if let Some(default) = default {
                    format!("{} [{}] ", prompt, default)
                } else {
                    format!("{}", prompt)
                };
                let input_builder = input::<String>()
                    .msg(&prompt)
                    .add_test(|value| value.len() > 0)
                    .repeat_msg(&prompt)
                    .err("Must be at least 1 character.  Please try again.");
                let value = if let Some(default) = variable.default().clone() {
                    input_builder.default(default.clone().to_owned()).get()
                } else {
                    input_builder.get()
                };
                context.insert(variable.identifier(), &value);
            } else if let Some(default) = default {
                context.insert(variable.identifier(), default.as_str());
            } else {
                return Err(ArchetypeError::ArchetypeInvalid);
            }
        }

        Ok(context)
    }
}

pub struct Module {
    config: ModuleConfig,
    archetype: Archetype,
}

impl Module {
    fn new(config: ModuleConfig, archetype: Archetype) -> Module {
        Module { config, archetype }
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
