use crate::config::rule::RuleConfig;
use crate::config::Answer;
use crate::ArchetypeError;
use std::fmt::{Display, Formatter};
use std::path::PathBuf;
use std::str::FromStr;
use std::{fmt, fs};

#[derive(Debug, Deserialize, Serialize)]
pub struct ArchetypeConfig {
    description: Option<String>,
    authors: Option<Vec<String>>,
    languages: Option<Vec<String>>,
    frameworks: Option<Vec<String>>,
    tags: Option<Vec<String>>,
    contents: Option<String>,
    #[serde(alias = "module")]
    modules: Option<Vec<ModuleConfig>>,
    #[serde(alias = "variable")]
    variables: Option<Vec<Variable>>,
    #[serde(alias = "path")]
    rules: Option<Vec<RuleConfig>>,
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

    pub fn with_description(mut self, description: &str) -> ArchetypeConfig {
        self.description = Some(description.into());
        self
    }

    pub fn add_author(&mut self, author: &str) {
        let authors = self.authors.get_or_insert_with(|| vec![]);
        authors.push(author.into());
    }

    pub fn with_author(mut self, author: &str) -> ArchetypeConfig {
        self.add_author(author);
        self
    }

    pub fn authors(&self) -> &[String] {
        self.authors.as_ref().map(|v| v.as_slice()).unwrap_or_default()
    }

    pub fn with_language(mut self, language: &str) -> ArchetypeConfig {
        self.add_language(language);
        self
    }

    pub fn add_language(&mut self, language: &str) {
        let languages = self.languages.get_or_insert_with(|| Vec::new());
        languages.push(language.to_owned());
    }

    pub fn languages(&self) -> &[String] {
        self.languages.as_ref().map(|r| r.as_slice()).unwrap_or_default()
    }

    pub fn with_tag(mut self, tag: &str) -> ArchetypeConfig {
        self.add_tag(tag);
        self
    }

    pub fn add_tag(&mut self, tag: &str) {
        let tags = self.tags.get_or_insert_with(|| Vec::new());
        tags.push(tag.to_owned());
    }

    pub fn tags(&self) -> &[String] {
        self.tags.as_ref().map(|r| r.as_slice()).unwrap_or_default()
    }

    pub fn with_framework(mut self, framework: &str) -> ArchetypeConfig {
        self.add_framework(framework);
        self
    }

    pub fn add_framework(&mut self, framework: &str) {
        let frameworks = self.frameworks.get_or_insert_with(|| Vec::new());
        frameworks.push(framework.to_owned());
    }

    pub fn frameworks(&self) -> &[String] {
        self.frameworks.as_ref().map(|r| r.as_slice()).unwrap_or_default()
    }

    pub fn with_module(mut self, module: ModuleConfig) -> ArchetypeConfig {
        self.add_module(module);
        self
    }

    pub fn add_module(&mut self, module: ModuleConfig) {
        let modules = self.modules.get_or_insert_with(|| vec![]);
        modules.push(module);
    }

    pub fn modules(&self) -> &[ModuleConfig] {
        self.modules.as_ref().map(|r| r.as_slice()).unwrap_or_default()
    }

    pub fn add_path_rule(&mut self, path_rule: RuleConfig) {
        let path_rules = self.rules.get_or_insert_with(|| vec![]);
        path_rules.push(path_rule);
    }

    pub fn with_path_rule(mut self, path_rule: RuleConfig) -> ArchetypeConfig {
        self.add_path_rule(path_rule);
        self
    }

    pub fn path_rules(&self) -> &[RuleConfig] {
        self.rules.as_ref().map(|pr| pr.as_slice()).unwrap_or_default()
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
        self.variables.as_ref().map(|v| v.as_slice()).unwrap_or_default()
    }

    pub fn with_contents(mut self, contents: &str) -> ArchetypeConfig {
        self.contents = Some(contents.into());
        self
    }

    pub fn contents_dir(&self) -> &str {
        self.contents.as_ref().map(|c| c.as_str()).unwrap_or("contents")
    }
}

impl Default for ArchetypeConfig {
    fn default() -> Self {
        ArchetypeConfig {
            description: None,
            authors: None,
            languages: None,
            frameworks: None,
            tags: None,
            contents: None,
            modules: None,
            rules: None,
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
    #[serde(alias = "location")]
    source: String,
    destination: String,
    #[serde(alias = "answer")]
    answers: Option<Vec<Answer>>,
}

impl ModuleConfig {
    pub fn new(source: &str, destination: &str) -> ModuleConfig {
        ModuleConfig {
            source: source.into(),
            destination: destination.into(),
            answers: None,
        }
    }

    pub fn source(&self) -> &str {
        self.source.as_str()
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
    #[serde(alias = "name")]
    #[serde(alias = "identifier")]
    #[serde(rename = "variable")]
    name: String,
    #[serde(alias = "default")]
    value: Option<String>,
    prompt: Option<String>,
    inherit: Option<bool>,
}

impl Variable {
    pub fn with_name(identifier: &str) -> VariableBuilder {
        VariableBuilder {
            variable: Variable {
                prompt: None,
                name: identifier.into(),
                value: None,
                inherit: None,
            },
        }
    }

    pub fn with_default(mut self, value: &str) -> Variable {
        self.value = Some(value.into());
        self
    }

    pub fn with_prompt(mut self, value: &str) -> Variable {
        self.prompt = Some(value.into());
        self
    }

    pub fn prompt(&self) -> Option<&str> {
        match &self.prompt {
            Some(prompt) => Some(&prompt),
            None => None,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn default(&self) -> Option<&str> {
        match &self.value {
            Some(value) => Some(value.as_str()),
            None => None,
        }
    }

    pub fn is_derived(&self) -> bool {
        self.prompt.is_none() && self.value.is_some()
    }

    pub fn is_inheritable(&self) -> bool {
        self.inherit.unwrap_or(false)
    }

    pub fn set_inheritable(&mut self, inheritable: Option<bool>) {
        self.inherit = inheritable
    }

    pub fn with_inheritable(mut self, inheritable: bool) -> Variable {
        self.set_inheritable(Some(inheritable));
        self
    }
}

pub struct VariableBuilder {
    variable: Variable,
}

impl VariableBuilder {
    pub fn with_prompt(mut self, prompt: &str) -> Variable {
        self.variable.prompt = Some(prompt.into());
        self.variable
    }

    pub fn with_default(mut self, default: &str) -> Variable {
        self.variable.value = Some(default.into());
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
                ModuleConfig::new("~/modules/jpa-persistence-module", "{{ name | train_case }}")
                    .with_answer(Answer::new("name", "{{ name }} Service")),
            )
            .with_variable(Variable::with_name("name").with_prompt("Application Name"))
            .with_variable(
                Variable::with_name("author")
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

            [[modules]]
            source = "~/modules/jpa-persistence-module"
            destination = "{{ name | train_case }}"

            [[modules.answers]]
            variable = "name"
            value = "{{ name }} Service"

            [[variables]]
            variable = "name"
            prompt = "Application Name"

            [[variables]]
            variable = "author"
            value = "Jimmie"
            prompt = "Author"
        "#
        );
        assert_eq!(output, expected);
        println!("{}", output);

        assert_eq!(config.modules().len(), 1);
    }

    #[test]
    fn test_archetype_config_from_string_plurals() {
        let expected = indoc!(
            r#"
            [[modules]]
            source = "~/modules/jpa-persistence-module"
            destination = "{{ name | train_case }}"

            [[modules.answers]]
            variable = "name"
            value = "{{ name }} Service"

            [[modules]]
            source = "~/modules/cli"
            destination = "{{ name | train_case }}"

            [[modules.answer]]
            variable = "name"
            value = "{{ name }} Service"

            [[variables]]
            identifier = "name"
            prompt = "Application Name"

            [[variables]]
            prompt = "Author"
            name = "author"
            value = "Jimmie"
            "#
        );
        let config = ArchetypeConfig::from_str(expected).unwrap();
        assert!(config.variables().contains(
            &Variable::with_name("author")
                .with_prompt("Author")
                .with_default("Jimmie")
        ));
    }

    #[test]
    fn test_archetype_config_from_string_singulars() {
        let expected = indoc!(
            r#"
            [[module]]
            source = "~/modules/jpa-persistence-module"
            destination = "{{ name | train_case }}"

            [[module.answer]]
            identifier = "name"
            value = "{{ name }} Service"

            [[module]]
            source = "~/modules/cli"
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
            value = "Jimmie"
            "#
        );
        let config = ArchetypeConfig::from_str(expected).unwrap();
        assert!(config.variables().contains(
            &Variable::with_name("author")
                .with_prompt("Author")
                .with_default("Jimmie")
        ));
    }

    #[test]
    fn test_archetype_load() {
        let config = ArchetypeConfig::load("archetypes/simple").unwrap();
        assert!(config
            .variables()
            .contains(&Variable::with_name("name").with_prompt("Application Name: ")));
    }

    #[test]
    fn test_archetype_to_string() {
        let config = ArchetypeConfig::load("archetypes/simple").unwrap();

        assert!(config
            .variables()
            .contains(&Variable::with_name("name").with_prompt("Application Name: ")));
    }

    #[test]
    fn test_answer_config_to_string() {
        let mut config = crate::config::AnswerConfig::default();
        config.add_answer_pair("fname", "Jimmie");
        config.add_answer_pair("lname", "Fulton");

        println!("{}", config);
    }
}
