use crate::actions::{ActionId, Action, LoopContext};
use std::path::Path;
use crate::{Archetect, Archetype, ArchetectError};
use crate::rules::RulesContext;
use linked_hash_map::LinkedHashMap;
use crate::config::VariableInfo;
use crate::template_engine::Context;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ForEachAction {
    #[serde(rename = "in")]
    source: ForEachSource,
    #[serde(rename = "do", alias = "actions")]
    actions: Vec<ActionId>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ForEachSource {
    #[serde(rename = "variable")]
    Variable(String),
    #[serde(rename = "split")]
    Split(SplitOptions)
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SplitOptions {
    input: String,
    separator: String,
}

impl ForEachAction {
    pub fn actions(&self) -> &Vec<ActionId> {
        self.actions.as_ref()
    }
}

impl Action for ForEachAction {
    fn execute<D: AsRef<Path>>(&self,
                               archetect: &Archetect,
                               archetype: &Archetype,
                               destination: D,
                               rules_context: &mut RulesContext,
                               answers: &LinkedHashMap<String, VariableInfo>,
                               context: &mut Context
    ) -> Result<(), ArchetectError> {
        match &self.source {
            ForEachSource::Variable(identifier) => {
                if let Some(value) = context.get(identifier) {
                    if let Some(items) = value.as_array() {
                        let mut context = context.clone();
                        let mut rules_context = rules_context.clone();
                        rules_context.set_break_triggered(false);

                        let mut loop_context = LoopContext{ index: 0 };

                        for item in items {
                            if rules_context.break_triggered() {
                                break;
                            }
                            context.insert("item", item);
                            context.insert("loop", &loop_context);
                            let action: ActionId = self.actions().into();
                            action.execute(archetect, archetype, destination.as_ref(), &mut rules_context, answers, &mut context)?;
                            loop_context.index = loop_context.index + 1;
                        }
                    } else {
                        let item = {
                            if value.is_number() {
                                value.as_i64().unwrap().to_string()
                            } else if value.is_boolean() {
                                value.as_bool().unwrap().to_string()
                            } else {
                                value.as_str().unwrap().to_string()
                            }
                        };

                        let mut context = context.clone();
                        let mut rules_context = rules_context.clone();
                        rules_context.set_break_triggered(false);
                        let loop_context = LoopContext{ index: 0 };
                        context.insert("item", &item);
                        context.insert("loop", &loop_context);

                        let action: ActionId = self.actions().into();
                        action.execute(archetect, archetype, destination.as_ref(), &mut rules_context, answers, &mut context)?;
                    }
                }
            }
            ForEachSource::Split(options) => {
                let input = archetect.render_string(&options.input, context)?;
                let splits = input.split(&options.separator);

                let mut context = context.clone();
                let mut rules_context = rules_context.clone();
                rules_context.set_break_triggered(false);

                let mut loop_context = LoopContext{ index: 0 };
                for split in splits {
                    if rules_context.break_triggered() {
                        break;
                    }
                    let split = split.trim();
                    if !split.is_empty() {
                        context.insert("item", split);
                        context.insert("loop", &loop_context);
                        let action: ActionId = self.actions().into();
                        action.execute(archetect, archetype, destination.as_ref(), &mut rules_context, answers, &mut context)?;
                        loop_context.index = loop_context.index + 1;
                    }
                }
            }
        }
        Ok(())
    }
}