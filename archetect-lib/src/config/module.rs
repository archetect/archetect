use linked_hash_map::LinkedHashMap;
use crate::config::AnswerInfo;
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ModuleInfo {
    #[serde(rename = "archetype")]
    Archetype(ArchetypeInfo),
    #[serde(rename = "template")]
    TemplateDirectory(TemplateInfo),
}

impl ModuleInfo {
    pub fn for_archetype(archetype: ArchetypeInfo) -> ModuleInfo {
        ModuleInfo::Archetype(archetype)
    }

    pub fn for_template(template: TemplateInfo) -> ModuleInfo {
        ModuleInfo::TemplateDirectory(template)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ArchetypeInfo {
    source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    destination: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    answers: Option<LinkedHashMap<String, AnswerInfo>>,
}

impl ArchetypeInfo {
    pub fn new<S: Into<String>>(source: S) -> ArchetypeInfo {
        ArchetypeInfo {
            source: source.into(),
            destination: None,
            answers: Default::default(),
        }
    }

    pub fn with_destination<D: Into<String>>(mut self, destination: D) -> ArchetypeInfo {
        self.destination = Some(destination.into());
        self
    }

    pub fn with_answer<I: Into<String>>(mut self, identifier: I, answer_info: AnswerInfo) -> ArchetypeInfo {
        let answers = self.answers.get_or_insert_with(|| LinkedHashMap::new());
        answers.insert(identifier.into(), answer_info);
        self
    }

    pub fn source(&self) -> &str {
        self.source.as_str()
    }

    pub fn destination(&self) -> Option<&str> {
        self.destination.as_ref().map(|d| d.as_str())
    }

    pub fn answers(&self) -> Option<&LinkedHashMap<String, AnswerInfo>> {
        self.answers.as_ref()
    }
}

impl From<ArchetypeInfo> for ModuleInfo {
    fn from(archetype: ArchetypeInfo) -> Self {
        ModuleInfo::Archetype(archetype)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TemplateInfo {
    source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    destination: Option<String>,
}

impl TemplateInfo {
    pub fn new<S: Into<String>>(source: S) -> TemplateInfo {
        TemplateInfo {
            source: source.into(),
            destination: None,
        }
    }

    pub fn with_destination<D: Into<String>>(mut self, destination: D) -> TemplateInfo {
        self.destination = Some(destination.into());
        self
    }

    pub fn source(&self) -> &str {
        self.source.as_str()
    }

    pub fn destination(&self) -> Option<&str> {
        self.destination.as_ref().map(|d| d.as_str())
    }
}

impl From<TemplateInfo> for ModuleInfo {
    fn from(template: TemplateInfo) -> Self {
        ModuleInfo::TemplateDirectory(template)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use indoc::indoc;
    use crate::config::module::ModuleInfo;

    #[test]
    fn test_serialize() {
        let expected = indoc! {
            r#"
            ---
            - template:
                source: ~/contents/
                destination: "."
            - archetype:
                source: "git@github.com:archetect/archetype-rust-cli.git"
                destination: "."
                answers:
                  organization:
                    value: Acme"#
        };

        let info = vec![
            ModuleInfo::for_template(TemplateInfo::new("~/contents/").with_destination(".")),
            ModuleInfo::for_archetype(ArchetypeInfo::new("git@github.com:archetect/archetype-rust-cli.git")
                .with_destination(".")
                .with_answer("organization", AnswerInfo::with_value("Acme").build())
            ),
        ];

        let yaml = serde_yaml::to_string(&info).unwrap();
        assert_eq!(expected, yaml);
        println!("{}", yaml);
    }

    #[test]
    fn test_deserialize() {
        let input = indoc! {
            r#"
            ---
            - template:
                source: "./contents/"
            - archetype:
                source: "git@github.com:archetect/archetype-rust-cli.git"
                destination: "."
                answers:
                  organization:
                    value: Acme
            "#
        };

        let _ = serde_yaml::from_str::<Vec<ModuleInfo>>(&input).unwrap();
    }

    #[test]
    fn test_deserialize_no_answers() {
        let input = indoc! {
            r#"
            ---
            - template:
                source: "contents"
            - archetype:
                source: "git@github.com:archetect/archetype-rust-cli.git"
                destination: "."
            "#
        };

        let _ = serde_yaml::from_str::<Vec<ModuleInfo>>(&input).unwrap();
    }

    #[test]
    fn test_deserialize_empty_answers() {
        let input = indoc! {
            r#"
            ---
            - template:
                source: "./contents/"
            - archetype:
                source: "git@github.com:archetect/archetype-rust-cli.git"
                destination: "."
                answers: {}
            "#
        };

        let _ = serde_yaml::from_str::<Vec<ModuleInfo>>(&input).unwrap();
    }
}