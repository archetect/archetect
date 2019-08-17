#[macro_use]
extern crate failure;
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
pub mod heck;
pub mod loggerv;
pub mod parser;
pub mod template_engine;
pub mod util;

use log::trace;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use read_input::prelude::*;
use template_engine::{Context, Tera};

use crate::config::ArchetypeConfig;
use crate::config::ModuleConfig;
use crate::config::{Answer, PatternType};
use crate::util::{Location, LocationError};
use failure::{Error, Fail};
use std::collections::HashMap;

pub struct Archetype {
    tera: Tera,
    config: ArchetypeConfig,
    path: PathBuf,
    modules: Vec<Module>,
}

impl Archetype {
    pub fn from_location(location: Location, offline: bool) -> Result<Archetype, ArchetypeError> {
        let tera = Tera::default();
        let mut result = match location {
            Location::LocalDirectory { path } => {
                let config = ArchetypeConfig::load(&path)?;
                Ok(Archetype {
                    tera,
                    config,
                    path,
                    modules: vec![],
                })
            }
            Location::RemoteGit { url: _, path } => {
                let config = ArchetypeConfig::load(&path)?;
                Ok(Archetype {
                    tera,
                    config,
                    path,
                    modules: vec![],
                })
            }
        };

        let mut modules = vec![];

        if let Ok(archetype) = &mut result {
            if let Some(module_configs) = archetype.configuration().modules() {
                for module_config in module_configs {
                    let location = Location::detect(module_config.location(), offline)?;
                    let module_archetype = Archetype::from_location(location, offline)?;
                    modules.push(Module::new(module_config.clone(), module_archetype));
                }
            }
            for module in modules {
                archetype.modules.push(module);
            }
        }

        result
    }

    pub fn configuration(&self) -> &ArchetypeConfig {
        &self.config
    }

    fn generate_internal<SRC: Into<PathBuf>, DEST: Into<PathBuf>>(
        &self,
        context: Context,
        source: SRC,
        destination: DEST,
    ) -> Result<(), Error> {
        let source = source.into();
        let destination = destination.into();

        if !source.is_dir() {
            panic!("This is not a valid directory");
        }

        let path_rules = self.configuration().path_rules().unwrap_or_default();

        'outer: for entry in fs::read_dir(&source)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                let name = self
                    .tera
                    .render_string(path.file_name().unwrap().to_str().unwrap(), context.clone())
                    .unwrap();
                let mut destination = destination.clone();
                destination.push(name);
                trace!("Generating {:?}", &destination);
                fs::create_dir_all(destination.as_path()).unwrap();
                self.generate_internal(context.clone(), path, destination)
                    .unwrap();
            } else if path.is_file() {
                for path_rule in path_rules {
                    if path_rule.pattern_type() == &PatternType::GLOB {
                        for pattern in path_rule.patterns() {
                            let matcher = glob::Pattern::new(pattern).unwrap();
                            if matcher.matches_path(&path) {
                                if !path_rule.filter().unwrap_or(true) {
                                    trace!("Copying    {:?}", &path);
                                    let name = self
                                        .tera
                                        .render_string(
                                            path.file_name().unwrap().to_str().unwrap(),
                                            context.clone(),
                                        )
                                        .unwrap();
                                    let template = fs::read_to_string(&path)?;
                                    let file_contents = self
                                        .tera
                                        .render_string(&template, context.clone())
                                        .unwrap();
                                    let destination = destination.clone().join(name);
                                    trace!("Generating {:?}", &destination);
                                    let mut output = File::create(&destination)?;
                                    output.write(file_contents.as_bytes()).unwrap();
                                    continue 'outer;
                                }
                            }
                        }
                    }
                }
                let name = self
                    .tera
                    .render_string(path.file_name().unwrap().to_str().unwrap(), context.clone())
                    .unwrap();
                let template = fs::read_to_string(&path)?;
                let file_contents = self.tera.render_string(&template, context.clone()).unwrap();
                let destination = destination.clone().join(name);
                trace!("Generating {:?}", &destination);
                let mut output = File::create(&destination)?;
                output.write(file_contents.as_bytes()).unwrap();
            }
        }

        Ok(())
    }

    pub fn generate<D: Into<PathBuf>>(
        &self,
        destination: D,
        context: Context,
    ) -> Result<(), ArchetypeError> {
        let destination = destination.into();
        fs::create_dir_all(&destination).unwrap();
        self.generate_internal(
            context.clone(),
            self.path.clone().join("archetype"),
            destination,
        )
        .unwrap();

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
                            answer.identifier().to_owned(),
                            self.tera
                                .render_string(answer.value(), context.clone())
                                .unwrap(),
                        ),
                    );
                }
            }
            let context = module.archetype.get_context(&answers)?;
            module.archetype.generate(destination, context)?;
        }
        Ok(())
    }

    pub fn get_context(
        &self,
        answers: &HashMap<String, Answer>,
    ) -> Result<Context, ArchetypeError> {
        let mut context = Context::new();

        for variable in self.config.variables() {
            let default = if let Some(answer) = answers.get(variable.identifier()) {
                if let Some(true) = answer.prompt() {
                    Some(
                        self.tera
                            .render_string(answer.value(), context.clone())
                            .unwrap(),
                    )
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
                    .msg(prompt)
                    .add_test(|value| value.len() > 0)
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

#[derive(Debug, Fail)]
pub enum ArchetypeError {
    #[fail(display = "Invalid Archetype")]
    ArchetypeInvalid,
    #[fail(display = "Invalid Answers config format")]
    InvalidAnswersConfig,
    #[fail(display = "Error saving Archetype Config file")]
    ArchetypeSaveFailed,
    #[fail(display = "Archetype Source Location error")]
    LocationError(LocationError),
}

impl From<LocationError> for ArchetypeError {
    fn from(cause: LocationError) -> Self {
        ArchetypeError::LocationError(cause)
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
