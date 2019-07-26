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

use log::{debug, info};
use std::collections::{HashMap};
use std::env;
use std::fmt;
use std::fmt::{Display, Formatter};
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use std::str::{self, FromStr};

use read_input::prelude::*;
use template_engine::{Context, Tera};

use failure::{Error, Fail};

pub mod parser;
pub mod template_engine;

pub trait Archetype {
    fn generate<D: Into<PathBuf>>(
        &self,
        destination: D,
        context: Context,
    ) -> Result<(), ArchetypeError>;

    fn get_context(&self, answers: &AnswerConfig) -> Result<Context, ArchetypeError>;

    // TODO: Add ability to extract variables used throughout an Archetype
//    fn get_variables(&self) -> Result<HashSet<String>, ArchetypeError>;
}

pub struct DirectoryArchetype {
    tera: Tera,
    config: ArchetypeConfig,
    directory: PathBuf,
}

impl DirectoryArchetype {
    pub fn new<D: Into<PathBuf>>(directory: D) -> Result<DirectoryArchetype, Error> {
        let tera = Tera::default();
        let directory = directory.into();
        if !directory.exists() {
            let dir_name = directory.to_str().unwrap();
            if dir_name.starts_with("http") || dir_name.starts_with("git") {
                // Use tempdir, instead
                let mut tmp = env::temp_dir();
                tmp.push("archetect");
                if tmp.exists() {
                    fs::remove_dir_all(&tmp)?;
                }
                debug!("Cloning {} to {}", directory.to_str().unwrap(), tmp.to_str().unwrap());
                fs::create_dir_all(&tmp).unwrap();

                Command::new("git")
                    .args(&[
                        "clone",
                        directory.to_str().unwrap(),
                        &format!("{}", tmp.display()),
                    ])
                    .output().unwrap();

                let config = ArchetypeConfig::load(&tmp)?;
                Ok(DirectoryArchetype {
                    tera,
                    config,
                    directory: tmp,
                })
            } else {
                return Err(ArchetypeError::ArchetypeInvalid.into());
            }
        } else {
            let config = ArchetypeConfig::load(&directory)?;
            Ok(DirectoryArchetype {
                tera,
                config,
                directory,
            })
        }
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
                info!("Generating {:?}", &destination);
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
                info!("Generating {:?}", &destination);
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
        self.generate_internal(
            context,
            self.directory.clone().join("archetype"),
            destination,
        )
        .unwrap();
        Ok(())
    }

    fn get_context(&self, answer_config: &AnswerConfig) -> Result<Context, ArchetypeError> {
        let mut context = Context::new();

        for var_config in self.config.variables() {
            if let Some(value) = answer_config.answers.get(&var_config.name) {
                context.insert(var_config.name(), value);
            } else {
                let prompt = if let Some(default) = var_config.default.clone() {
                    format!("{} [{}] ", var_config.prompt, default)
                } else {
                    format!("{}", var_config.prompt)
                };
                let input_builder = input::<String>()
                    .msg(prompt)
                    .add_test(|value| value.len() > 0)
                    .err("Must be at least 1 character.  Please try again.");
                let value = if let Some(default) = var_config.default.clone() {
                    input_builder.default(default.clone()).get()
                } else {
                    input_builder.get()
                };

                context.insert(var_config.name(), &value);
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

#[derive(Debug, Deserialize, Serialize)]
pub struct ArchetypeConfig {
    variables: Vec<VariableEntry>,
}

impl ArchetypeConfig {
    pub fn load<P: Into<PathBuf>>(path: P) -> Result<ArchetypeConfig, ArchetypeError> {
        let mut path = path.into();
        if path.is_dir() {
            path.push("archetype.toml");
        }
        if !path.exists() {
            Err(ArchetypeError::ArchetypeInvalid)
        } else {
            let config = fs::read_to_string(&path).unwrap();
            let config = toml::de::from_str::<ArchetypeConfig>(&config).unwrap();
            Ok(config)
        }
    }

    pub fn save<P: Into<PathBuf>>(&self, path: P) -> Result<(), ArchetypeError> {
        let mut path = path.into();
        if path.is_dir() {
            path.push("archetype.toml");
        }
        fs::write(path, self.to_string().as_bytes()).unwrap();

        Ok(())
    }

    pub fn add_variable<P: Into<String>, N: Into<String>>(&mut self, prompt: P, name: N) {
        self.variables.push(VariableEntry::new(prompt, name));
    }

    pub fn add_variable_with_default<P: Into<String>, N: Into<String>, D: Into<String>>(
        &mut self,
        prompt: P,
        name: N,
        default_value: D,
    ) {
        self.variables
            .push(VariableEntry::new(prompt, name).with_default(default_value));
    }

    pub fn variables(&self) -> &[VariableEntry] {
        &self.variables
    }
}

impl Default for ArchetypeConfig {
    fn default() -> Self {
        ArchetypeConfig {
            variables: Vec::new(),
        }
    }
}

impl Display for ArchetypeConfig {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match toml::ser::to_string_pretty(self) {
            Ok(config) => write!(f, "{}", config),
            Err(_) => Err(fmt::Error),
        }
    }
}

impl FromStr for ArchetypeConfig {
    type Err = ArchetypeError;

    fn from_str(config: &str) -> Result<Self, Self::Err> {
        let result = toml::de::from_str::<ArchetypeConfig>(config);
        println!("{:?}", result);

        result.map_err(|_| ArchetypeError::ArchetypeInvalid)
    }
}

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq)]
pub struct VariableEntry {
    prompt: String,
    name: String,
    default: Option<String>,
}

impl VariableEntry {
    pub fn new<P: Into<String>, N: Into<String>>(prompt: P, name: N) -> VariableEntry {
        VariableEntry {
            prompt: prompt.into(),
            name: name.into(),
            default: None,
        }
    }

    pub fn with_default<D: Into<String>>(mut self, value: D) -> VariableEntry {
        self.default = Some(value.into());
        self
    }

    pub fn prompt(&self) -> &str {
        &self.prompt
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn default(&self) -> Option<&str> {
        match &self.default {
            Some(value) => Some(value.as_str()),
            None => None,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AnswerConfig {
    answers: HashMap<String, String>,
}

impl AnswerConfig {
    pub fn load<P: Into<PathBuf>>(path: P) -> Result<AnswerConfig, ArchetypeError> {
        let path = path.into();
        if path.is_dir() {
            let dot_answers = path.clone().join(".answers.toml");
            if dot_answers.exists() {
                let config = fs::read_to_string(dot_answers).unwrap();
                let config = toml::de::from_str::<AnswerConfig>(&config).unwrap();
                return Ok(config);
            }

            let answers = path.clone().join("answers.toml");
            if answers.exists() {
                let config = fs::read_to_string(answers).unwrap();
                let config = toml::de::from_str::<AnswerConfig>(&config).unwrap();
                return Ok(config);
            }
        } else {
            let config = fs::read_to_string(path).unwrap();
            let config = toml::de::from_str::<AnswerConfig>(&config).unwrap();
            return Ok(config);
        }

        Err(ArchetypeError::InvalidAnswersConfig)
    }

    pub fn add_answer<K: Into<String>, V: Into<String>>(&mut self, key: K, value: V) {
        self.answers.insert(key.into(), value.into());
    }

    pub fn answers(&self) -> &HashMap<String, String> {
        &self.answers
    }
}

impl Default for AnswerConfig {
    fn default() -> Self {
        AnswerConfig {
            answers: HashMap::new(),
        }
    }
}
impl FromStr for AnswerConfig {
    type Err = ArchetypeError;

    fn from_str(config: &str) -> Result<Self, Self::Err> {
        toml::de::from_str::<AnswerConfig>(config).map_err(|_| ArchetypeError::ArchetypeInvalid)
    }
}

impl Display for AnswerConfig {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match toml::ser::to_string_pretty(self) {
            Ok(config) => write!(f, "{}", config),
            Err(_) => Err(fmt::Error),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AnswerEntry {
    variable: String,
    answer: String,
}

impl AnswerEntry {
    pub fn new<V: Into<String>, A: Into<String>>(variable: V, answer: A) -> AnswerEntry {
        AnswerEntry {
            variable: variable.into(),
            answer: answer.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use indoc::indoc;

    #[test]
    fn test_archetype_config_to_string() {
        let mut config = ArchetypeConfig::default();
        config.add_variable("Application Name", "name");
        config.add_variable_with_default("Author", "author", "Jimmie");

        let output = config.to_string();

        let expected = indoc!(r#"
            [[variables]]
            prompt = 'Application Name'
            name = 'name'

            [[variables]]
            prompt = 'Author'
            name = 'author'
            default = 'Jimmie'
        "#);
        assert_eq!(output, expected);
        println!("{}", output);
    }

    #[test]
    fn test_archetype_config_from_string() {
        let expected = indoc!(r#"
            [[variables]]
            prompt = "Application Name"
            name = "name"

            [[variables]]
            prompt = "Author"
            name = "author"
            default = "Jimmie"
            "#);
        let config = ArchetypeConfig::from_str(expected).unwrap();
        assert!(config
            .variables()
            .contains(&VariableEntry::new("Author", "author").with_default("Jimmie")));
    }

    #[test]
    fn test_archetype_load() {
        let config = ArchetypeConfig::load("templates/simple").unwrap();
        assert!(config
            .variables()
            .contains(&VariableEntry::new("Application Name: ", "name")));
    }

    #[test]
    fn test_archetype_to_string() {
        let config = ArchetypeConfig::load("templates/simple").unwrap();

        assert!(config
            .variables()
            .contains(&VariableEntry::new("Application Name: ", "name")));
    }

    #[test]
    fn test_answer_config_to_string() {
        let mut config = AnswerConfig::default();
        config.add_answer("fname", "Jimmie");
        config.add_answer("lname", "Fulton");

        println!("{}", config);
    }
}
