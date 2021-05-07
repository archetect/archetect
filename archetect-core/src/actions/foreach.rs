use std::path::Path;

use linked_hash_map::LinkedHashMap;

use crate::actions::{Action, ActionId, LoopContext};
use crate::config::VariableInfo;
use crate::rules::RulesContext;
use crate::template_engine::Context;
use crate::{Archetect, ArchetectError, Archetype};

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
    Split(SplitOptions),
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
    fn execute<D: AsRef<Path>>(
        &self,
        archetect: &Archetect,
        archetype: &Archetype,
        destination: D,
        rules_context: &mut RulesContext,
        answers: &LinkedHashMap<String, VariableInfo>,
        context: &mut Context,
    ) -> Result<(), ArchetectError> {
        match &self.source {
            ForEachSource::Variable(identifier) => {
                if let Some(value) = context.get(identifier) {
                    if let Some(items) = value.as_array() {
                        let mut context = context.clone();
                        let mut rules_context = rules_context.clone();
                        rules_context.set_break_triggered(false);

                        let mut loop_context = LoopContext::new();

                        for item in items {
                            if rules_context.break_triggered() {
                                break;
                            }
                            context.insert("item", item);
                            context.insert("loop", &loop_context);
                            let action: ActionId = self.actions().into();
                            action.execute(
                                archetect,
                                archetype,
                                destination.as_ref(),
                                &mut rules_context,
                                answers,
                                &mut context,
                            )?;
                            loop_context.increment();
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
                        let loop_context = LoopContext::new();
                        context.insert("item", &item);
                        context.insert("loop", &loop_context);

                        let action: ActionId = self.actions().into();
                        action.execute(
                            archetect,
                            archetype,
                            destination.as_ref(),
                            &mut rules_context,
                            answers,
                            &mut context,
                        )?;
                    }
                }
            }
            ForEachSource::Split(options) => {
                let input = archetect.render_string(&options.input, context)?;
                let splits = input.split(&options.separator);

                let mut context = context.clone();
                let mut rules_context = rules_context.clone();
                rules_context.set_break_triggered(false);

                let mut loop_context = LoopContext::new();
                for split in splits {
                    if rules_context.break_triggered() {
                        break;
                    }
                    let split = split.trim();
                    if !split.is_empty() {
                        context.insert("item", split);
                        context.insert("loop", &loop_context);
                        let action: ActionId = self.actions().into();
                        action.execute(
                            archetect,
                            archetype,
                            destination.as_ref(),
                            &mut rules_context,
                            answers,
                            &mut context,
                        )?;
                        loop_context.increment();
                    }
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ForAction {
    #[serde(flatten)]
    options: ForOptions,
    #[serde(rename = "do")]
    actions: Vec<ActionId>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ForOptions {
    #[serde(rename = "item")]
    Item {
        #[serde(rename = "in")]
        identifier: String,
        name: Option<String>,
        value: Option<String>,
    },
    #[serde(rename = "split")]
    Split {
        #[serde(rename = "in")]
        input: String,
        #[serde(rename = "sep")]
        separator: Option<String>,
        name: Option<String>,
        value: Option<String>,
    },
}

impl ForAction {
    pub fn actions(&self) -> &Vec<ActionId> {
        self.actions.as_ref()
    }
}

impl Action for ForAction {
    fn execute<D: AsRef<Path>>(
        &self,
        archetect: &Archetect,
        archetype: &Archetype,
        destination: D,
        rules_context: &mut RulesContext,
        answers: &LinkedHashMap<String, VariableInfo>,
        context: &mut Context,
    ) -> Result<(), ArchetectError> {
        match &self.options {
            ForOptions::Item {
                identifier,
                name,
                value,
            } => {
                let format = value;
                if let Some(value) = context.get(identifier) {
                    if let Some(items) = value.as_array() {
                        let mut context = context.clone();
                        let mut rules_context = rules_context.clone();
                        rules_context.set_break_triggered(false);

                        let mut loop_context = LoopContext::new();

                        for item in items {
                            if rules_context.break_triggered() {
                                break;
                            }
                            context.insert(name.clone().unwrap_or("item".to_owned()).as_ref(), item);
                            if let Some(format) = format {
                                let format = archetect.render_string(format, &context)?;
                                context.insert(name.clone().unwrap_or("item".to_owned()).as_ref(), &format);
                            }
                            context.insert("loop", &loop_context);
                            let action: ActionId = self.actions().into();
                            action.execute(
                                archetect,
                                archetype,
                                destination.as_ref(),
                                &mut rules_context,
                                answers,
                                &mut context,
                            )?;
                            loop_context.increment();
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
                        let loop_context = LoopContext::new();
                        context.insert(name.clone().unwrap_or("item".to_owned()).as_str(), &item);
                        if let Some(format) = format {
                            let format = archetect.render_string(format, &context)?;
                            context.insert(name.clone().unwrap_or("item".to_owned()).as_ref(), &format);
                        }
                        context.insert("loop", &loop_context);

                        let action: ActionId = self.actions().into();
                        action.execute(
                            archetect,
                            archetype,
                            destination.as_ref(),
                            &mut rules_context,
                            answers,
                            &mut context,
                        )?;
                    }
                }
            }
            ForOptions::Split {
                input,
                separator,
                name,
                value,
            } => {
                let format = value;
                let input = archetect.render_string(input, context)?;
                let separator = separator.clone().unwrap_or(",".to_owned());
                let splits = input.split(&separator);

                let mut context = context.clone();
                let mut rules_context = rules_context.clone();
                rules_context.set_break_triggered(false);

                let mut loop_context = LoopContext::new();
                for split in splits {
                    if rules_context.break_triggered() {
                        break;
                    }
                    let split = split.trim();
                    if !split.is_empty() {
                        context.insert(name.clone().unwrap_or("item".to_owned()).as_str(), split);
                        if let Some(format) = format {
                            let format = archetect.render_string(format, &context)?;
                            context.insert(name.clone().unwrap_or("item".to_owned()).as_ref(), &format);
                        }
                        context.insert("loop", &loop_context);
                        let action: ActionId = self.actions().into();
                        action.execute(
                            archetect,
                            archetype,
                            destination.as_ref(),
                            &mut rules_context,
                            answers,
                            &mut context,
                        )?;
                        loop_context.increment();
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use serde_yaml;

    use crate::actions::foreach::{ForAction, ForOptions};
    use crate::actions::ActionId;

    #[test]
    fn test_serialize_for_item() {
        let action = ForAction {
            options: ForOptions::Item {
                identifier: "products".to_owned(),
                name: Some("product".to_owned()),
                value: None,
            },
            actions: vec![ActionId::Break],
        };

        println!("{}", serde_yaml::to_string(&action).unwrap());
    }

    #[test]
    fn test_serialize_for_split() {
        let action = ForAction {
            options: ForOptions::Split {
                input: "{{ products }}".to_owned(),
                separator: Some(",".to_owned()),
                name: Some("product".to_owned()),
                value: None,
            },
            actions: vec![ActionId::Break],
        };

        println!("{}", serde_yaml::to_string(&action).unwrap());
    }
}
