use crate::config::path::PathRuleConfig;
use crate::config::Answer;
use crate::ArchetypeError;
use std::fmt::{Display, Formatter};
use std::path::PathBuf;
use std::str::FromStr;
use std::{fmt, fs};

#[derive(Debug, Deserialize, Serialize)]
pub struct ArchetypeConfig {
    description: Option<String>,
    languages: Option<Vec<String>>,
    frameworks: Option<Vec<String>>,
    tags: Option<Vec<String>>,
    contents: Option<String>,
    #[serde(rename = "module")]
    modules: Option<Vec<ModuleConfig>>,
    #[serde(rename = "variable")]
    variables: Option<Vec<Variable>>,
    #[serde(rename = "path")]
    path_rules: Option<Vec<PathRuleConfig>>,
}

impl ArchetypeConfig {
    pub fn new() -> ArchetypeConfig {
        ArchetypeConfig::default()
    }

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

    pub fn languages(&self) -> Option<&[String]> {
        self.languages.as_ref().map(|r| r.as_slice())
    }

    pub fn with_tag<T: Into<String>>(mut self, tag: T) -> ArchetypeConfig {
        self.add_tag(tag);
        self
    }

    pub fn add_tag<T: Into<String>>(&mut self, tag: T) {
        let tags = self.tags.get_or_insert_with(|| Vec::new());
        tags.push(tag.into());
    }

    pub fn tags(&self) -> Option<&[String]> {
        self.tags.as_ref().map(|r| r.as_slice())
    }

    pub fn with_framework<F: Into<String>>(mut self, framework: F) -> ArchetypeConfig {
        self.add_framework(framework);
        self
    }

    pub fn add_framework<F: Into<String>>(&mut self, framework: F) {
        let frameworks = self.frameworks.get_or_insert_with(|| Vec::new());
        frameworks.push(framework.into());
    }

    pub fn frameworks(&self) -> Option<&[String]> {
        self.frameworks.as_ref().map(|r| r.as_slice())
    }

    pub fn with_module(mut self, module: ModuleConfig) -> ArchetypeConfig {
        self.add_module(module);
        self
    }

    pub fn add_module(&mut self, module: ModuleConfig) {
        let modules = self.modules.get_or_insert_with(|| vec![]);
        modules.push(module);
    }

    pub fn modules(&self) -> Option<&[ModuleConfig]> {
        self.modules.as_ref().map(|r| r.as_slice())
    }

    pub fn add_path_rule(&mut self, path_rule: PathRuleConfig) {
        let path_rules = self.path_rules.get_or_insert_with(|| vec![]);
        path_rules.push(path_rule);
    }

    pub fn with_path_rule(mut self, path_rule: PathRuleConfig) -> ArchetypeConfig {
        self.add_path_rule(path_rule);
        self
    }

    pub fn path_rules(&self) -> Option<&[PathRuleConfig]> {
        self.path_rules.as_ref().map(|pr| pr.as_slice())
    }

    pub fn add_variable(&mut self, variable: Variable) {
        let variables = self.variables.get_or_insert_with(|| vec![]);
        variables.push(variable);
    }

    pub fn with_variable(mut self, variable: Variable) -> ArchetypeConfig {
        self.add_variable(variable);
        self
    }

    pub fn variables(&self) -> &[Variable] {
        self.variables
            .as_ref()
            .map(|v| v.as_slice())
            .unwrap_or_default()
    }

    pub fn with_contents<C: Into<String>>(mut self, contents: C) -> ArchetypeConfig {
        self.contents = Some(contents.into());
        self
    }

    pub fn contents(&self) -> Option<&str> {
        self.contents.as_ref().map(|r| r.as_str())
    }
}

impl Default for ArchetypeConfig {
    fn default() -> Self {
        ArchetypeConfig {
            description: None,
            languages: None,
            frameworks: None,
            tags: None,
            contents: None,
            modules: None,
            path_rules: None,
            variables: None,
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModuleConfig {
    location: String,
    destination: String,
    #[serde(rename = "answer")]
    answers: Option<Vec<Answer>>,
}

impl ModuleConfig {
    pub fn new<L: Into<String>, D: Into<String>>(location: L, destination: D) -> ModuleConfig {
        ModuleConfig {
            location: location.into(),
            destination: destination.into(),
            answers: None,
        }
    }

    pub fn location(&self) -> &str {
        self.location.as_str()
    }

    pub fn destination(&self) -> &str {
        self.destination.as_str()
    }

    pub fn with_answer(mut self, answer: Answer) -> ModuleConfig {
        self.add_answer(answer);
        self
    }

    pub fn add_answer(&mut self, answer: Answer) {
        let answers = self.answers.get_or_insert_with(|| vec![]);
        answers.push(answer);
    }

    pub fn answers(&self) -> Option<&[Answer]> {
        self.answers.as_ref().map(|r| r.as_slice())
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
        VariableBuilder {
            variable: Variable {
                prompt: None,
                name: identifier.into(),
                default: None,
            },
        }
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
            .with_module(
                ModuleConfig::new(
                    "~/modules/jpa-persistence-module",
                    "{{ name | train_case }}",
                )
                .with_answer(Answer::new("name", "{{ name }} Service")),
            )
            .with_variable(Variable::with_identifier("name").with_prompt("Application Name"))
            .with_variable(
                Variable::with_identifier("author")
                    .with_prompt("Author")
                    .with_default("Jimmie"),
            );

        let output = config.to_string();

        let expected = indoc!(
            r#"
            description = "Simple REST Service"
            languages = ["Java"]
            frameworks = ["Spring", "Hessian"]
            tags = ["Service", "REST"]

            [[module]]
            location = "~/modules/jpa-persistence-module"
            destination = "{{ name | train_case }}"

            [[module.answer]]
            identifier = "name"
            value = "{{ name }} Service"

            [[variable]]
            prompt = "Application Name"
            name = "name"

            [[variable]]
            prompt = "Author"
            name = "author"
            default = "Jimmie"
        "#
        );
        assert_eq!(output, expected);
        println!("{}", output);
    }

    #[test]
    fn test_archetype_config_from_string() {
        let expected = indoc!(
            r#"
            [[variable]]
            prompt = "Application Name"
            name = "name"

            [[variable]]
            prompt = "Author"
            name = "author"
            default = "Jimmie"
            "#
        );
        let config = ArchetypeConfig::from_str(expected).unwrap();
        assert!(config.variables().contains(
            &Variable::with_identifier("author")
                .with_prompt("Author")
                .with_default("Jimmie")
        ));
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
