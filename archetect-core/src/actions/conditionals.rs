use std::path::Path;

use linked_hash_map::LinkedHashMap;

use log::trace;

use crate::actions::{Action, ActionId};
use crate::config::VariableInfo;
use crate::rules::RulesContext;
use crate::{Archetect, ArchetectError, Archetype};
use crate::vendor::tera::Context;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IfAction {
    #[serde(flatten)]
    condition: Condition,
    #[serde(rename = "then", alias = "actions")]
    then_actions: Vec<ActionId>,
    #[serde(rename = "else")]
    else_actions: Option<Vec<ActionId>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Condition {
    #[serde(rename = "all-of")]
    AllOf(Vec<Condition>),
    #[serde(rename = "conditions")]
    Conditions(Vec<Condition>),
    #[serde(rename = "equals")]
    Equals(String, String),
    #[serde(rename = "matches")]
    Matches(String, String),
    #[serde(rename = "is-blank")]
    IsBlank(String),
    #[serde(rename = "is-empty")]
    IsEmpty(String),
    #[serde(rename = "path-exists")]
    PathExists(String),
    #[serde(rename = "is-file")]
    IsFile(String),
    #[serde(rename = "is-directory")]
    IsDirectory(String),
    #[serde(rename = "switch-enabled")]
    SwitchEnabled(String),
    #[serde(rename = "not")]
    Not(Box<Condition>),
    #[serde(rename = "any-of")]
    AnyOf(Vec<Condition>),
}

impl IfAction {
    pub fn then_actions(&self) -> &Vec<ActionId> {
        self.then_actions.as_ref()
    }

    pub fn else_actions(&self) -> Option<&Vec<ActionId>> {
        self.else_actions.as_ref()
    }
}

impl Condition {
    pub fn evaluate<D: AsRef<Path>>(
        &self,
        archetect: &mut Archetect,
        archetype: &Archetype,
        destination: D,
        context: &Context,
    ) -> Result<bool, ArchetectError> {
        match self {
            Condition::IsBlank(input) => {
                if let Some(value) = context.get(input) {
                    if let Some(string) = value.as_str() {
                        return Ok(string.trim().is_empty());
                    }
                }
                Ok(false)
            }
            Condition::IsEmpty(input) => {
                if let Some(value) = context.get(input) {
                    if let Some(string) = value.as_str() {
                        return Ok(string.trim().is_empty());
                    }
                }
                Ok(false)
            }
            Condition::PathExists(path) => {
                let path = archetect.render_string(path, context)?;
                let path = destination.as_ref().join(path);
                Ok(path.exists())
            }
            Condition::IsFile(path) => {
                let path = archetect.render_string(path, context)?;
                let path = destination.as_ref().join(path);
                let exists = path.exists() && path.is_file();
                trace!("[File Exists] {}: {}", path.display(), exists);
                Ok(exists)
            }
            Condition::IsDirectory(path) => {
                let path = archetect.render_string(path, context)?;
                let path = destination.as_ref().join(path);
                Ok(path.exists() && path.is_dir())
            }
            Condition::SwitchEnabled(switch) => Ok(archetect.switches().contains(switch)),
            Condition::Not(condition) => {
                let value = condition.evaluate(archetect, archetype, destination, context)?;
                Ok(!value)
            }
            Condition::Equals(left, right) => {
                let left = archetect.render_string(left, context)?;
                let right = archetect.render_string(right, context)?;
                return Ok(left.eq(&right));
            }
            Condition::Matches(left, right) => {
                let left = archetect.render_string(left, context)?;
                let right = archetect.render_string(right, context)?;
                return Ok(left.eq(&right));
            }
            Condition::AnyOf(conditions) => {
                for condition in conditions {
                    let value = condition.evaluate(archetect, archetype, destination.as_ref(), context)?;
                    if value {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            Condition::AllOf(conditions) => {
                for condition in conditions {
                    let value = condition.evaluate(archetect, archetype, destination.as_ref(), context)?;
                    if !value {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            Condition::Conditions(conditions) => {
                for condition in conditions {
                    let value = condition.evaluate(archetect, archetype, destination.as_ref(), context)?;
                    if !value {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
        }
    }
}

impl Action for IfAction {
    fn execute<D: AsRef<Path>>(
        &self,
        archetect: &mut Archetect,
        archetype: &Archetype,
        destination: D,
        rules_context: &mut RulesContext,
        answers: &LinkedHashMap<String, VariableInfo>,
        context: &mut Context,
    ) -> Result<(), ArchetectError> {
        if self.condition.evaluate(archetect, archetype, destination.as_ref(), context)? {
            let action: ActionId = self.then_actions().into();
            action.execute(
                archetect,
                archetype,
                destination.as_ref(),
                rules_context,
                answers,
                context,
            )?;
        } else {
            if let Some(actions) = self.else_actions() {
                let action: ActionId = actions[..].into();
                action.execute(
                    archetect,
                    archetype,
                    destination.as_ref(),
                    rules_context,
                    answers,
                    context,
                )?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::actions::conditionals::{Condition, IfAction};
    use crate::actions::render::{DirectoryOptions, RenderAction};
    use crate::actions::ActionId;

    #[test]
    pub fn test_serialize() -> Result<(), serde_yaml::Error> {
        let action = IfAction {
            condition: Condition::AllOf(vec![
                Condition::IsFile("example.txt".to_owned()),
                Condition::IsDirectory("src/main/java".to_owned()),
                Condition::PathExists("{{ service }}".to_owned()),
                Condition::Equals("{{ one }}".to_owned(), "{{ two }}".to_owned()),
            ]),
            then_actions: vec![ActionId::Render(RenderAction::Directory(DirectoryOptions::new(".")))],
            else_actions: None,
        };

        let yaml = serde_yaml::to_string(&action)?;
        println!("{}", yaml);

        Ok(())
    }
}
