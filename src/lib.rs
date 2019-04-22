#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate tera;

use std::{fs, io};
use std::fs::{DirEntry, File};
use std::io::Write;
use std::path::PathBuf;
use read_input::prelude::*;

use tera::{Context, Tera};

use read_input::InputBuilderOnce;
use read_input::shortcut::input_d;

pub trait Archetype {
    fn generate<D: Into<PathBuf>>(&self, destination: D, context: Context) -> Result<(), ArchetypeError>;

    fn get_context(&self) -> Result<Context, ArchetypeError>;
}

pub struct DirectoryArchetype {
    tera: Tera,
    config: Config,
    directory: PathBuf,
}

impl DirectoryArchetype {
    pub fn new<D: Into<PathBuf>>(directory: D) -> Result<DirectoryArchetype, ArchetypeError> {
        let directory = directory.into();
        let tera = Tera::default();
        let config = fs::read_to_string(directory.clone().join("archetype.toml")).unwrap();
        let config = toml::de::from_str::<Config>(&config).unwrap();
        Ok(DirectoryArchetype { tera, config, directory })
    }


    fn generate_internal<SRC: Into<PathBuf>, DEST: Into<PathBuf>>(&self, context: Context, source: SRC, destination: DEST) -> Result<(), io::Error> {
        let source = source.into();
        let destination = destination.into();

        if !source.is_dir() {
            panic!("This is not a valid directory");
        }

        for entry in fs::read_dir(&source)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                let name = self.tera.render_string(path.file_name().unwrap().to_str().unwrap(), context.clone()).unwrap();
                let mut destination = destination.clone();
                destination.push(name);
                println!("Generating {:?}", &destination);
                fs::create_dir_all(destination.as_path());
                self.generate_internal(context.clone(), path, destination).unwrap();
            } else if path.is_file() {
                let name = self.tera.render_string(path.file_name().unwrap().to_str().unwrap(), context.clone()).unwrap();
                let template = fs::read_to_string(&path)?;
                let file_contents = self.tera.render_string(&template, context.clone()).unwrap();
                let destination = destination.clone().join(name);
                println!("Generating {:?}", &destination);
                let mut output = File::create(&destination)?;
                output.write(file_contents.as_bytes()).unwrap();
            }
        }

        Ok(())
    }

    fn render(&self, contents: &str, context: &Context) {
        let context = context.clone();
        let renderer = Renderer::new()
    }
}

impl Archetype for DirectoryArchetype {
    fn generate<D: Into<PathBuf>>(&self, destination: D, context: Context) -> Result<(), ArchetypeError> {
        let destination = destination.into();
        fs::create_dir_all(&destination).unwrap();
        self.generate_internal(context, self.directory.clone().join("contents"), destination);
        Ok(())

    }

    fn get_context(&self) ->  Result<Context, ArchetypeError> {
        let mut context = Context::new();

        for varConfig in self.config.variables() {
            let prompt = if let Some(default) = varConfig.default.clone() {
                format!("{} [{}] ", varConfig.prompt, default)
            } else {
                format!("{}", varConfig.prompt)
            };
            let mut input_builder = input::<String>().msg(prompt)
                .add_test(|value| value.len() > 0)
                .err("Must be at least 1 character.  Please try again.");
            let value = if let Some(default) = varConfig.default.clone() {
                input_builder.default(default.clone()).get()
            } else {
                input_builder.get()
            };

            context.insert(varConfig.name(), &value);
        }

        Ok(context)
    }
}

#[derive(Debug)]
pub enum ArchetypeError {
    InvalidArchetype,
}


#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    variables: Vec<VariableConfig>,
}

impl Config {
    pub fn add_variable<P: Into<String>, N: Into<String>>(&mut self, prompt: P, name: N) {
        self.variables.push(VariableConfig::new(prompt, name));
    }

    pub fn add_variable_with_default<P: Into<String>, N: Into<String>, D: Into<String>>(&mut self, prompt: P, name: N, default_value: D) {
        self.variables.push(VariableConfig::new(prompt, name).with_default(default_value));
    }

    pub fn variables(&self) -> &[VariableConfig] {
        &self.variables
    }
}

impl Default for Config {
    fn default() -> Self {
        Config { variables: Vec::new() }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct VariableConfig {
    prompt: String,
    name: String,
    default: Option<String>,
}

impl VariableConfig {
    pub fn new<P: Into<String>, N: Into<String>>(prompt: P, name: N) -> VariableConfig {
        VariableConfig { prompt: prompt.into(), name: name.into(), default: None }
    }

    pub fn with_default<D: Into<String>>(mut self, value: D) -> VariableConfig {
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
            None => None
        }
    }
}

#[cfg(test)]
mod tests {
    use std::env::current_dir;

    use tera::{Context, Tera};

    use super::*;

    #[test]
    fn it_works() -> Result<(), ArchetypeError> {

        let dst = current_dir().map(|p| p.join("result")).unwrap();
        fs::remove_dir_all(&dst);

        let archetype = DirectoryArchetype::new("./templates/simple")?;
        let mut context = archetype.get_context()?;

        context.insert("first", "one");
        context.insert("second", "two");
        context.insert("third", "three");
        context.insert("author", "Jimmie Fulton <jimmie.fulton@gmail.com>");
        context.insert("name", "HydraMQ");

        archetype.generate(dst, context);


        Ok(())
    }

    #[test]
    fn test_serialize_config() {
        let mut config = Config::default();
        config.add_variable("Application Name", "name");
        config.add_variable_with_default("Author", "author", "Jimmie");

        let output = toml::ser::to_string_pretty(&config).unwrap();

        let expected =
            r#"[[variables]]
prompt = 'Application Name'
name = 'name'

[[variables]]
prompt = 'Author'
name = 'author'
default = 'Jimmie'
"#;
        assert_eq!(output, expected);
        println!("{}", output);
    }
}
