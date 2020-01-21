use std::path::Path;

use linked_hash_map::LinkedHashMap;
use log::{debug, error, info, trace, warn};
use serde::{Deserialize, Serialize};

use crate::{Archetect, ArchetectError, Archetype};
use crate::actions::render::RenderAction;
use crate::config::{AnswerInfo, VariableInfo};
use crate::template_engine::Context;
use crate::actions::conditionals::IfAction;
use crate::rendering::Renderable;
use crate::rules::RulesContext;
use crate::actions::rules::{RuleType};
use crate::actions::foreach::ForEachAction;

pub mod conditionals;
pub mod foreach;
pub mod render;
pub mod rules;
pub mod set;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ActionId {
    #[serde(rename = "set")]
    Set(LinkedHashMap<String, VariableInfo>),
    #[serde(rename = "scope")]
    Scope(Vec<ActionId>),
    #[serde(rename = "actions")]
    Actions(Vec<ActionId>),
    #[serde(rename = "render")]
    Render(RenderAction),
    #[serde(rename = "for-each")]
    ForEach(ForEachAction),
    #[serde(rename = "if")]
    If(IfAction),
    #[serde(rename = "rules")]
    Rules(Vec<RuleType>),

    // Logging
    #[serde(rename = "trace")]
    LogTrace(String),
    #[serde(rename = "debug")]
    LogDebug(String),
    #[serde(rename = "info")]
    LogInfo(String),
    #[serde(rename = "warn")]
    LogWarn(String),
    #[serde(rename = "error")]
    LogError(String),
}

impl ActionId {
    pub fn execute<D: AsRef<Path>>(&self,
                                   archetect: &Archetect,
                                   archetype: &Archetype,
                                   destination: D,
                                   rules_context: &mut RulesContext,
                                   answers: &LinkedHashMap<String, AnswerInfo>,
                                   context: &mut Context,
    ) -> Result<(), ArchetectError> {
        let destination = destination.as_ref();
        match self {
            ActionId::Set(variables) => {
                set::populate_context(archetect, variables, answers, context)?;
            }
            ActionId::Render(action) => { action.execute(archetect, archetype, destination, rules_context, answers, context)? }
            ActionId::Actions(action_ids) => {
                for action_id in action_ids {
                    action_id.execute(archetect, archetype, destination, rules_context, answers, context)?;
                }
            }

            // Logging
            ActionId::LogTrace(message) => { trace!("{}", message.render(&archetect, context)?) }
            ActionId::LogDebug(message) => { debug!("{}", message.render(&archetect, context)?) }
            ActionId::LogInfo(message) => { info!("{}", message.render(&archetect, context)?) }
            ActionId::LogWarn(message) => { warn!("{}", message.render(&archetect, context)?) }
            ActionId::LogError(message) => { error!("{}", message.render(&archetect, context)?) }

            ActionId::Scope(actions) => {
                let mut rules_context = rules_context.clone();
                let mut scope_context = context.clone();
                let action = ActionId::from(actions.as_ref());
                action.execute(archetect, archetype, destination, &mut rules_context, answers, &mut scope_context)?;
            }
            ActionId::If(action) => { action.execute(archetect, archetype, destination, rules_context, answers, context)? }
            ActionId::Rules(actions) => {
                for action in actions {
                    action.execute(archetect, archetype, destination, rules_context, answers, context)?;
                }
            }
            ActionId::ForEach(action) => {
                action.execute(archetect, archetype, destination, rules_context, answers, context)?;
            }
        }

        Ok(())
    }
}

impl From<Vec<ActionId>> for ActionId {
    fn from(action_ids: Vec<ActionId>) -> Self {
        ActionId::Actions(action_ids)
    }
}

impl From<&[ActionId]> for ActionId {
    fn from(action_ids: &[ActionId]) -> Self {
        let actions: Vec<ActionId> = action_ids.iter().map(|i| i.to_owned()).collect();
        ActionId::Actions(actions)
    }
}

pub trait Action {
    fn execute<D: AsRef<Path>>(&self,
                               archetect: &Archetect,
                               archetype: &Archetype,
                               destination: D,
                               rules_context: &mut RulesContext,
                               answers: &LinkedHashMap<String, AnswerInfo>,
                               context: &mut Context,
    ) -> Result<(), ArchetectError>;
}


#[cfg(test)]
mod tests {
    use crate::actions::render::{ArchetypeOptions, DirectoryOptions};
    
    use super::*;

    #[test]
    fn test_serialize() {
        let actions = vec![
            ActionId::LogWarn("Warning!!".to_owned()),
            ActionId::Render(RenderAction::Directory(DirectoryOptions::new("."))),
            ActionId::Render(RenderAction::Archetype(ArchetypeOptions::new("git@github.com:archetect/archetype-rust-cli.git"))),
        ];

        let yaml = serde_yaml::to_string(&actions).unwrap();
        println!("{}", yaml);
    }
}
