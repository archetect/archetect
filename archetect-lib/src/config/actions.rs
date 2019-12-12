use crate::config::AnswerInfo;
use linked_hash_map::LinkedHashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Actions {
    #[serde(rename = "render")]
    Render(RenderAction),
    #[serde(rename = "actions")]
    ActionsList(Vec<Actions>),
    #[serde(rename = "iterate")]
    Iterate(IterateAction),
}

pub trait Action {}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RenderAction {
    #[serde(rename = "type")]
    render_type: RenderType,
    source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    destination: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    answers: Option<LinkedHashMap<String, AnswerInfo>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum RenderType {
    #[serde(rename = "directory")]
    Directory,
    #[serde(rename = "archetype")]
    Archetype,
}

impl RenderAction {
    pub fn new<S: Into<String>>(render_type: RenderType, source: S) -> RenderAction {
        RenderAction {
            render_type,
            source: source.into(),
            destination: None,
            answers: Default::default(),
        }
    }

    pub fn with_destination<D: Into<String>>(mut self, destination: D) -> RenderAction {
        self.destination = Some(destination.into());
        self
    }
}

impl Action for RenderAction {}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IterateAction {
    over: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    answers: Option<LinkedHashMap<String, AnswerInfo>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    actions: Option<Vec<Actions>>,
}

impl IterateAction {
    pub fn new<O: Into<String>>(over: O) -> IterateAction {
        IterateAction {
            over: over.into(),
            answers: None,
            actions: None,
        }
    }

    pub fn with_answer<I: Into<String>>(mut self, identifier: I, answer_info: AnswerInfo) -> IterateAction {
        let answers = self.answers.get_or_insert_with(|| LinkedHashMap::new());
        answers.insert(identifier.into(), answer_info);
        self
    }

    pub fn with_action(mut self, action: Actions) -> IterateAction {
        let actions = self.actions.get_or_insert_with(|| Vec::new());
        actions.push(action);
        self
    }
}

impl Action for IterateAction {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize() {
        let actions = vec![
            Actions::Iterate(
                IterateAction::new("customers")
                    .with_answer("customer", AnswerInfo::with_value("{{ item }}").build())
                    .with_action(Actions::Render(
                        RenderAction::new(RenderType::Archetype, "git@github.com:archetect/archetype-rust-cli.git")
                            .with_destination("{{ artifact_id }}"),
                    )),
            ),
            Actions::Render(RenderAction::new(RenderType::Directory, ".")),
            Actions::Render(
                RenderAction::new(RenderType::Archetype, "git@github.com:archetect/archetype-rust-cli.git")
                    .with_destination("{{ artifact_id }}"),
            ),
            Actions::ActionsList(vec![]),
        ];

        let yaml = serde_yaml::to_string(&actions).unwrap();
        println!("{}", yaml);
    }
}
