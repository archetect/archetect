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

use crate::config::Answer;
use crate::config::ArchetypeConfig;
use crate::util::Location;
use failure::{Error, Fail};
use std::collections::HashMap;

pub trait Archetype {
    fn generate<D: Into<PathBuf>>(
        &self,
        destination: D,
        context: Context,
    ) -> Result<(), ArchetypeError>;

    fn get_context(&self, answers: &HashMap<String, Answer>) -> Result<Context, ArchetypeError>;

    // TODO: Add ability to extract variables used throughout an Archetype
    //    fn get_variables(&self) -> Result<HashSet<String>, ArchetypeError>;
}

pub struct DirectoryArchetype {
    tera: Tera,
    config: ArchetypeConfig,
    path: PathBuf,
}

impl DirectoryArchetype {
    pub fn from_location(location: Location, offline: bool) -> Result<DirectoryArchetype, Error> {
        let tera = Tera::default();
        let result = match location {
            Location::LocalDirectory { path } => {
                let config = ArchetypeConfig::load(&path)?;
                Ok(DirectoryArchetype { tera, config, path })
            }
            Location::RemoteGit { url: _, path } => {
                let config = ArchetypeConfig::load(&path)?;
                Ok(DirectoryArchetype { tera, config, path })
            }
        };

        if let Ok(archetype) = &result {
            if let Some(modules) = archetype.configuration().modules() {
                for module in modules {
                    Location::detect(module.location(), offline).unwrap();
                }
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

        for entry in fs::read_dir(&source)? {
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
}

impl Archetype for DirectoryArchetype {
    fn generate<D: Into<PathBuf>>(
        &self,
        destination: D,
        context: Context,
    ) -> Result<(), ArchetypeError> {
        let destination = destination.into();
        fs::create_dir_all(&destination).unwrap();
        self.generate_internal(context, self.path.clone().join("archetype"), destination)
            .unwrap();
        Ok(())
    }

    fn get_context(&self, answers: &HashMap<String, Answer>) -> Result<Context, ArchetypeError> {
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

#[derive(Debug, Fail)]
pub enum ArchetypeError {
    #[fail(display = "Invalid Archetype")]
    ArchetypeInvalid,
    #[fail(display = "Invalid Answers config format")]
    InvalidAnswersConfig,
    #[fail(display = "Error saving Archetype Config file")]
    ArchetypeSaveFailed,
}

#[cfg(test)]
mod tests {
    use url::Url;

    #[test]
    fn test_url_as_http() {
        let url = Url::parse("https://github.com/jimmiebfulton/archetect.git").unwrap();
        println!("{:?}", url);
        println!("{}", url.scheme());
        println!("{}", url.host_str().unwrap());
        println!("{}", url.path());
    }
}
