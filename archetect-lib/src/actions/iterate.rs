use linked_hash_map::LinkedHashMap;
use crate::config::AnswerInfo;
use crate::{Archetect, ArchetectError, Archetype};
use crate::actions::{ActionId, Action};
use std::path::{Path};
use crate::template_engine::Context;
use crate::rules::RulesContext;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IterateAction {
    over: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    answers: Option<LinkedHashMap<String, AnswerInfo>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    actions: Option<Vec<ActionId>>,
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

    pub fn with_action(mut self, action: ActionId) -> IterateAction {
        let actions = self.actions.get_or_insert_with(|| Vec::new());
        actions.push(action);
        self
    }
}

impl Action for IterateAction {

    fn execute<D: AsRef<Path>>(&self,
               archetect: &Archetect,
               archetype: &Archetype,
               destination: D,
               rules_context: &mut RulesContext,
               answers: &LinkedHashMap<String, AnswerInfo>,
               context: &mut Context
    ) -> Result<(), ArchetectError> {
        let destination = destination.as_ref();
        
        if let Some(actions) = self.actions.as_ref() {
            for action in actions {

                action.execute(archetect, archetype, destination, rules_context, answers, context)?;
            }
        }
        Ok(())
    }
}