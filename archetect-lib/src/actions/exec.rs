use std::collections::hash_map::RandomState;
use std::path::Path;
use std::process::Command;

use linked_hash_map::LinkedHashMap;
use log::{debug, warn};

use crate::{Archetect, ArchetectError, Archetype};
use crate::actions::Action;
use crate::config::VariableInfo;
use crate::rules::RulesContext;
use crate::template_engine::Context;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExecAction {
    command: String,
    args: Option<Vec<String>>,
    env: Option<LinkedHashMap<String, String>>,
    cwd: Option<String>,
}

impl ExecAction {
    pub fn args(&self) -> Option<&Vec<String>> {
        self.args.as_ref()
    }

    pub fn env(&self) -> Option<&LinkedHashMap<String, String>> {
        self.env.as_ref()
    }
}

impl Action for ExecAction {
    fn execute<D: AsRef<Path>>(&self,
                               archetect: &Archetect,
                               _archetype: &Archetype,
                               destination: D,
                               _rules_context: &mut RulesContext,
                               _answers: &LinkedHashMap<String, VariableInfo, RandomState>,
                               context: &mut Context,
    ) -> Result<(), ArchetectError> {
        let mut command = Command::new(&self.command);

        if let Some(args) = self.args() {
            for arg in args {
                command.arg(archetect.render_string(arg, context)?);
            }
        }

        if let Some(env) = self.env() {
            for (key, value) in env  {
                command.env(
                    archetect.render_string(key, context)?,
                    archetect.render_string(value, context)?,
                );
            }
        }

        if let Some(cwd) = &self.cwd {
            if let Ok(cwd) = shellexpand::full(cwd) {
                let cwd = Path::new(cwd.as_ref());
                if cwd.is_relative() {
                    command.current_dir(destination.as_ref()
                        .join(archetect.render_string(cwd.display().to_string().as_str(), context)?));
                } else {
                    command.current_dir(archetect.render_string(cwd.display().to_string().as_str(), context)?);
                }
            }
        } else {
            command.current_dir(destination);
        }

        debug!("[exec] Executing: {:?}", command);
        match command.status() {
            Ok(status) => {
                debug!("[exec] Status: {}", status.code().unwrap());
            }
            Err(error) => {
                warn!("[exec] Error: {}", error);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::actions::exec::ExecAction;
    use serde_yaml;
    use linked_hash_map::LinkedHashMap;

    #[test]
    fn test_serialize() {
        let mut env = LinkedHashMap::new();
        env.insert("M2_HOME".to_owned(), "~/.m2".to_owned());
        env.insert("MAVEN_HOME".to_owned(), "/usr/bin".to_owned());

        let mut foo = LinkedHashMap::new();
        foo.insert("exmple".to_owned(), ());
        let action = ExecAction {
            command: "mvn".to_string(),
            args: Some(vec!["install".to_owned()]),
            env: Some(env),
            cwd: None
        };

        println!("{}", serde_yaml::to_string(&action).unwrap());
    }
}