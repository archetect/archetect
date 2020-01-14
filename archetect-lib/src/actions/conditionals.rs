use std::path::Path;

use linked_hash_map::LinkedHashMap;
use serde::{Deserialize, Serialize};

use log::{trace};

use crate::{Archetect, ArchetectError, Archetype};
use crate::actions::{Action, ActionId};
use crate::config::VariableInfo;
use crate::template_engine::Context;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IfAction {
    conditions: Vec<Condition>,
    actions: Vec<ActionId>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Condition {
    #[serde(rename = "is-not-blank")]
    IsNotBlank(String),
    #[serde(rename = "path-exists")]
    PathExists(String),
    #[serde(rename = "path-not-exists")]
    PathNotExists(String),
    #[serde(rename = "is-file")]
    IsFile(String),
    #[serde(rename = "is-directory")]
    IsDirectory(String),
}

impl Condition {
    pub fn evaluate<D: AsRef<Path>>(&self,
                                    archetect: &Archetect,
                                    _archetype: &Archetype,
                                    destination: D,
                                    context: &Context,
    ) -> Result<bool, ArchetectError> {
        match self {
            Condition::IsNotBlank(input) => {
                if let Some(value) = context.get(input) {
                    if let Some(string) = value.as_str() {
                        return Ok(!string.trim().is_empty());
                    }
                }
                Ok(false)
            }
            Condition::PathExists(path) => {
                let path = archetect.render_string(path, context)?;
                let path = destination.as_ref().join(path);
                Ok(path.exists())
            }
            Condition::PathNotExists(path) => {
                let path = archetect.render_string(path, context)?;
                let path = destination.as_ref().join(path);
                Ok(!path.exists())
            }
            Condition::IsFile(path) => {
                let path = archetect.render_string(path, context)?;
                let path = destination.as_ref().join(path);
                let exists = path.exists() && path.is_file();
                trace!("[File Exists] {}: {}", path.to_str().unwrap(), exists);
                Ok(exists)
            }
            Condition::IsDirectory(path) => {
                let path = archetect.render_string(path, context)?;
                let path = destination.as_ref().join(path);
                Ok(path.exists() && path.is_dir())
            }
        }
    }
}

impl Action for IfAction {
    fn execute<D: AsRef<Path>>(&self,
                               archetect: &Archetect,
                               archetype: &Archetype,
                               destination: D,
                               answers: &LinkedHashMap<String, VariableInfo>,
                               context: &mut Context,
    ) -> Result<(), ArchetectError> {
        for condition in &self.conditions {
            if condition.evaluate(archetect, archetype, destination.as_ref(), context)? == false {
                return Ok(());
            }
        }

        for action in &self.actions {
            action.execute(archetect, archetype, destination.as_ref(), answers, context)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::actions::ActionId;
    use crate::actions::conditionals::{Condition, IfAction};
    use crate::actions::render::{DirectoryOptions, RenderAction};

    #[test]
    pub fn test_serialize() {
        let action = IfAction {
            conditions: vec![
                Condition::IsNotBlank("organization".to_owned()),
                Condition::IsFile("example.txt".to_owned()),
                Condition::IsDirectory("src/main/java".to_owned()),
                Condition::PathExists("{{ service }}".to_owned()),
                Condition::PathNotExists("{{ service }}".to_owned()),
            ],
            actions: vec![ActionId::Render(RenderAction::Directory(DirectoryOptions::new(".")))],
        };

        let yaml = serde_yaml::to_string(&action).unwrap();
        println!("{}", yaml);
    }
}