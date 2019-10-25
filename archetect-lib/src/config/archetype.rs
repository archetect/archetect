use crate::config::rule::RuleConfig;
use crate::config::{AnswerInfo, ModuleInfo};
use crate::config::VariableInfo;
use crate::ArchetypeError;
use linked_hash_map::LinkedHashMap;
use std::fmt::{Display, Formatter};
use std::path::PathBuf;
use std::str::FromStr;
use std::{fmt, fs};

#[derive(Debug, Deserialize, Serialize)]
pub struct ArchetypeConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    authors: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    languages: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    frameworks: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "LinkedHashMap::is_empty")]
    variables: LinkedHashMap<String, VariableInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    contents: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    modules: Vec<ModuleInfo>,
    #[serde(alias = "path")]
    #[serde(skip_serializing_if = "Option::is_none")]
    rules: Option<Vec<RuleConfig>>,
}

impl ArchetypeConfig {
    pub fn new() -> ArchetypeConfig {
        ArchetypeConfig::default()
    }

    pub fn load<P: Into<PathBuf>>(path: P) -> Result<ArchetypeConfig, ArchetypeError> {
        let mut path = path.into();
        if path.is_dir() {
            let candidates = vec!["archetype.yml", "archetype.yaml"];
            for candidate in candidates {
                let config_file = path.join(candidate);
                if config_file.exists() {
                    path = config_file;
                }
            }
        }
        if !path.exists() {
            Err(ArchetypeError::ArchetypeInvalid)
        } else {
            let config = fs::read_to_string(&path).unwrap();
            let config = serde_yaml::from_str::<ArchetypeConfig>(&config).unwrap();
            Ok(config)
        }
    }

    pub fn save<P: Into<PathBuf>>(&self, path: P) -> Result<(), ArchetypeError> {
        let mut path = path.into();
        if path.is_dir() {
            path.push("archetype.yaml");
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

    pub fn with_module<M: Into<ModuleInfo>>(mut self, module: M) -> ArchetypeConfig {
        self.add_module(module.into());
        self
    }

    pub fn add_module<M: Into<ModuleInfo>>(&mut self, module: M) {
        self.modules.push(module.into());
    }

    pub fn modules(&self) -> &[ModuleInfo] {
        self.modules.as_slice()
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

    pub fn add_variable<I: Into<String>>(&mut self, identifier: I, variable_info: VariableInfo) {
        self.variables.insert(identifier.into(), variable_info);
    }

    pub fn with_variable<I: Into<String>>(mut self, identifier: I, variable_info: VariableInfo) -> ArchetypeConfig {
        self.add_variable(identifier, variable_info);
        self
    }

    pub fn variables(&self) -> &LinkedHashMap<String, VariableInfo> {
        &self.variables
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
            modules: Vec::new(),
            rules: None,
            variables: LinkedHashMap::new(),
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
    #[serde(skip_serializing_if = "LinkedHashMap::is_empty")]
    answers: LinkedHashMap<String, AnswerInfo>,
}

impl ModuleConfig {
    pub fn new(source: &str, destination: &str) -> ModuleConfig {
        ModuleConfig {
            source: source.into(),
            destination: destination.into(),
            answers: LinkedHashMap::new(),
        }
    }

    pub fn source(&self) -> &str {
        self.source.as_str()
    }

    pub fn destination(&self) -> &str {
        self.destination.as_str()
    }

    pub fn with_answer<I: Into<String>>(mut self, identifier: I, answer_info: AnswerInfo) -> ModuleConfig {
        self.add_answer(identifier.into(), answer_info);
        self
    }

    pub fn add_answer<I: Into<String>>(&mut self, identifier: I, answer_info: AnswerInfo) {
        self.answers.insert(identifier.into(), answer_info);
    }

    pub fn answers(&self) -> &LinkedHashMap<String, AnswerInfo> {
        &self.answers
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use indoc::indoc;
    use crate::config::{PatternType, RuleAction, ArchetypeInfo};

    #[test]
    fn test_serialize_to_yaml() {
        let config = ArchetypeConfig::default()
            .with_description("Simple REST Service")
            .with_language("Java")
            .with_framework("Spring")
            .with_framework("Hessian")
            .with_tag("Service")
            .with_tag("REST")
            .with_module(
                ArchetypeInfo::new("~/modules/jpa-persistence-module").with_destination("{{ name | train_case }}")
                    .with_answer("name", AnswerInfo::with_value("{{ name }} Service").build()),
            )
            .with_variable("organization", VariableInfo::with_prompt("Organization: ").build())
            .with_variable("author", VariableInfo::with_prompt("Author: ").build())
            .with_path_rule(RuleConfig::new(PatternType::GLOB).with_pattern("*.jpg").with_action(RuleAction::COPY))
            ;

        let output = serde_yaml::to_string(&config).unwrap();
        println!("{}", output);
    }

    #[test]
    fn test_deserialize_from_yaml() {
        let input = indoc! {
            r#"
            ---
            description: Simple REST Service
            languages: ["Java"]
            frameworks: ["Spring", "Hessian"]
            tags: ["Service", "REST"]

            variables:
              author:
                prompt: "Author: "
              organization:
                prompt: "Organization: "
                default: "Acme Inc"
                
            modules:
              - template:
                  source: "contents"
              - archetype:
                  source: ~/modules/jpa-persistence-module
                  destination: "{{ name | train_case }}"
                  answers:
                    name:
                      value: "{{ name }} Service"
            "#
        };

        let config = serde_yaml::from_str::<ArchetypeConfig>(&input).unwrap();

        assert_eq!(config.variables().len(), 2);
        assert_eq!(config.variables().get("author").unwrap().prompt().unwrap(), "Author: ");
        assert_eq!(config.variables().get("organization").unwrap().prompt().unwrap(), "Organization: ");
        assert_eq!(config.variables().get("organization").unwrap().default().unwrap(), "Acme Inc");
    }

    #[test]
    fn test_archetype_load() {
        let config = ArchetypeConfig::load("archetypes/arch-java-maven").unwrap();
        assert_eq!(config.variables().get("name").unwrap(), &VariableInfo::with_prompt("Application Name: ").build());
    }
}
