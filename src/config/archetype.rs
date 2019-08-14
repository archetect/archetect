use std::path::PathBuf;
use crate::ArchetypeError;
use std::{fs, fmt};
use std::fmt::{Display, Formatter};
use std::str::FromStr;

#[derive(Debug, Deserialize, Serialize)]
pub struct ArchetypeConfig {
    description: Option<String>,
    languages: Option<Vec<String>>,
    frameworks: Option<Vec<String>>,
    tags: Option<Vec<String>>,
    variables: Vec<Variable>,
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



    pub fn with_description<D: Into<String>>(mut self, description: D) -> ArchetypeConfig {
        self.description = Some(description.into());
        self
    }

    pub fn with_language<L: Into<String>>(mut self, language: L) -> ArchetypeConfig {
        self.add_language(language);
        self
    }

    pub fn add_language<L: Into<String>>(&mut self, language: L) {
        let languages = self.languages.get_or_insert_with(|| Vec::new());
        languages.push(language.into());
    }

    pub fn languages(&self) -> Option<&Vec<String>> {
        self.languages.as_ref()
    }

    pub fn with_tag<T: Into<String>>(mut self, tag: T) -> ArchetypeConfig {
        self.add_tag(tag);
        self
    }

    pub fn add_tag<T: Into<String>>(&mut self, tag: T) {
        let tags = self.tags.get_or_insert_with(|| Vec::new());
        tags.push(tag.into());
    }

    pub fn tags(&self) -> Option<&Vec<String>> {
        self.tags.as_ref()
    }

    pub fn with_framework<F: Into<String>>(mut self, framework: F) -> ArchetypeConfig {
        self.add_framework(framework);
        self
    }

    pub fn add_framework<F: Into<String>>(&mut self, framework: F) {
        let frameworks = self.frameworks.get_or_insert_with(|| Vec::new());
        frameworks.push(framework.into());
    }

    pub fn frameworks(&self) -> Option<&Vec<String>> {
        self.frameworks.as_ref()
    }

    pub fn with_variable(mut self, variable: Variable) -> ArchetypeConfig {
        self.variables.push(variable);
        self
    }

    pub fn add_variable(&mut self, variable: Variable) {
        self.variables.push(variable);
    }

    pub fn variables(&self) -> &[Variable] {
        &self.variables
    }
}

impl Default for ArchetypeConfig {
    fn default() -> Self {
        ArchetypeConfig {
            description: None,
            languages: None,
            frameworks: None,
            tags: None,
            variables: Vec::new(),
        }
    }
}

impl Display for ArchetypeConfig {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match toml::ser::to_string(self) {
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
pub struct Variable {
    prompt: Option<String>,
    name: String,
    default: Option<String>,
}

impl Variable {
    pub fn with_identifier<I: Into<String>>(identifier: I) -> VariableBuilder {
        VariableBuilder { variable: Variable { prompt: None, name: identifier.into(), default: None } }
    }

    pub fn with_default<D: Into<String>>(mut self, value: D) -> Variable {
        self.default = Some(value.into());
        self
    }

    pub fn with_prompt<P: Into<String>>(mut self, value: P) -> Variable {
        self.prompt = Some(value.into());
        self
    }

    pub fn prompt(&self) -> Option<&str> {
        match &self.prompt {
            Some(prompt) => Some(&prompt),
            None => None,
        }
    }

    pub fn identifier(&self) -> &str {
        &self.name
    }

    pub fn default(&self) -> Option<&str> {
        match &self.default {
            Some(value) => Some(value.as_str()),
            None => None,
        }
    }
}

pub struct VariableBuilder {
    variable: Variable,
}

impl VariableBuilder {
    pub fn with_prompt<P: Into<String>>(mut self, prompt: P) -> Variable {
        self.variable.prompt = Some(prompt.into());
        self.variable
    }

    pub fn with_default<D: Into<String>>(mut self, default: D) -> Variable {
        self.variable.default = Some(default.into());
        self.variable
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use indoc::indoc;

    #[test]
    fn test_archetype_config_to_string() {
        let config = ArchetypeConfig::default()
            .with_description("Simple REST Service")
            .with_language("Java")
            .with_framework("Spring")
            .with_framework("Hessian")
            .with_tag("Service")
            .with_tag("REST")
            .with_variable(
                Variable::with_identifier("name")
                    .with_prompt("Application Name"))
            .with_variable(
                Variable::with_identifier("author")
                    .with_prompt("Author")
                    .with_default("Jimmie"))
            ;

        let output = config.to_string();

        let expected = indoc!(r#"
            description = "Simple REST Service"
            languages = ["Java"]
            frameworks = ["Spring", "Hessian"]
            tags = ["Service", "REST"]

            [[variables]]
            prompt = "Application Name"
            name = "name"

            [[variables]]
            prompt = "Author"
            name = "author"
            default = "Jimmie"
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
            .contains(&Variable::with_identifier("author").with_prompt("Author").with_default("Jimmie")));
    }

    #[test]
    fn test_archetype_load() {
        let config = ArchetypeConfig::load("templates/simple").unwrap();
        assert!(config
            .variables()
            .contains(&Variable::with_identifier("name").with_prompt("Application Name: ")));
    }

    #[test]
    fn test_archetype_to_string() {
        let config = ArchetypeConfig::load("templates/simple").unwrap();

        assert!(config
            .variables()
            .contains(&Variable::with_identifier("name").with_prompt("Application Name: ")));
    }

    #[test]
    fn test_answer_config_to_string() {
        let mut config = crate::config::AnswerConfig::default();
        config.add_answer_pair("fname", "Jimmie");
        config.add_answer_pair("lname", "Fulton");

        println!("{}", config);
    }
}