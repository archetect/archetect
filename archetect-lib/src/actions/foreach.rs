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
                        for item in items {
                            let mut context = context.clone();
                            context.insert("item", item);

                            let mut rules_context = rules_context.clone();

                            for action in &self.actions {
                                action.execute(archetect, archetype, destination.as_ref(), &mut rules_context, answers, &mut context)?;
                            }
                        }
                    } else if let Some(item) = value.as_str() {
                        let mut context = context.clone();
                        context.insert("item", item);

                        let mut context = context.clone();
                        context.insert("item", item);

                        let mut rules_context = rules_context.clone();
                        rules_context.set_break_triggered(false);

                        let mut loop_context = LoopContext{ index: 0 };
                        for action in &self.actions {
                            context.insert("loop", &loop_context);
                            action.execute(archetect, archetype, destination.as_ref(), &mut rules_context, answers, &mut context)?;
                            if rules_context.break_triggered() {
                                break;
                            }
                            loop_context.index = loop_context + 1;
                        }
                    }
                }
            }
            ForEachSource::Split(options) => {
                let input = archetect.render_string(&options.input, context)?;
                let splits = input.split(&options.separator);

                for split in splits {
                    let split = split.trim();
                    if !split.is_empty() {
                        let mut context = context.clone();
                        context.insert("item", split);

                        let mut rules_context = rules_context.clone();
                        rules_context.set_break_triggered(false);

                        let mut loop_context = LoopContext{ index: 0 };
                        for action in &self.actions {
                            context.insert("loop", &loop_context);
                            action.execute(archetect, archetype, destination.as_ref(), &mut rules_context, answers, &mut context)?;
                            if rules_context.break_triggered() {
                                break;
                            }
                            loop_context.index = loop_context + 1;
                        }
                    }
                }
            }
        }
        Ok(())
    }
}