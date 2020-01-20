use crate::ArchetypeError;
use std::path::PathBuf;
use std::{fs};
use crate::actions::ActionId;

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
    #[serde(skip_serializing_if = "Option::is_none", alias = "actions")]
    script: Option<Vec<ActionId>>,
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

    pub fn with_action(mut self, action: ActionId) -> ArchetypeConfig {
        self.add_action(action);
        self
    }

    pub fn add_action(&mut self, action: ActionId) {
        let actions = self.script.get_or_insert_with(|| Vec::new());
        actions.push(action);
    }

    pub fn actions(&self) -> &[ActionId] {
        self.script.as_ref().map(|r| r.as_slice()).unwrap_or_default()
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
            script: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::iterate::IterateAction;
    use crate::actions::render::{RenderAction, DirectoryOptions};
    use crate::config::AnswerInfo;

    #[test]
    fn test_serialize_to_yaml() {
        let config = ArchetypeConfig::default()
            .with_description("Simple REST Service")
            .with_language("Java")
            .with_framework("Spring")
            .with_framework("Hessian")
            .with_tag("Service")
            .with_tag("REST")
            .with_action(ActionId::Iterate(
                IterateAction::new("services")
                    .with_answer("service", AnswerInfo::with_value("{{ item | snake_case }}").build())
                    .with_action(ActionId::Render(RenderAction::Directory(DirectoryOptions::new(".")))),
            ))
            ;

        let output = serde_yaml::to_string(&config).unwrap();
        println!("{}", output);
    }

//    #[test]
//    fn test_deserialize_from_yaml() {
//        let input = indoc! {
//            r#"
//            ---
//            description: Simple REST Service
//            languages: ["Java"]
//            frameworks: ["Spring", "Hessian"]
//            tags: ["Service", "REST"]
//            requires: ^1.2.0
//
//            variables:
//              author:
//                prompt: "Author: "
//              organization:
//                prompt: "Organization: "
//                default: "Acme Inc"
//
//            modules:
//              - template:
//                  source: "contents"
//              - archetype:
//                  source: ~/modules/jpa-persistence-module
//                  destination: "{{ name | train_case }}"
//                  answers:
//                    name:
//                      value: "{{ name }} Service"
//
//            "#
//        };
//
//        let config = serde_yaml::from_str::<ArchetypeConfig>(&input).unwrap();
//
//        assert_eq!(config.variables().unwrap().len(), 2);
//        assert_eq!(config.variables().unwrap().get("author").unwrap().prompt().unwrap(), "Author: ");
//        assert_eq!(
//            config.variables().unwrap().get("organization").unwrap().prompt().unwrap(),
//            "Organization: "
//        );
//        assert_eq!(
//            config.variables().unwrap().get("organization").unwrap().default().unwrap(),
//            "Acme Inc"
//        );
//    }

//    #[test]
//    fn test_archetype_load() {
//        let config = ArchetypeConfig::load("archetypes/arch-java-maven").unwrap();
//        assert_eq!(
//            config.variables().unwrap().get("name").unwrap(),
//            &VariableInfo::with_prompt("Application Name: ").build()
//        );
//    }
}
