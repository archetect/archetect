use std::path::{Path};

use linked_hash_map::LinkedHashMap;

use crate::{Archetect, ArchetectError, Archetype};
use crate::actions::{Action, ActionId};
use crate::config::AnswerInfo;
use crate::template_engine::Context;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SplitAction {
    input: String,
    separator: String,
    actions: Vec<ActionId>,
}

impl Action for SplitAction {
    fn execute<D: AsRef<Path>>(&self,
               archetect: &Archetect,
               archetype: &Archetype,
               destination: D,
               answers: &LinkedHashMap<String, AnswerInfo>,
               context: &mut Context
    ) -> Result<(), ArchetectError> {
        let destination = destination.as_ref();
        let input = archetect.render_string(&self.input, context)?;
        let splits = input.split(&self.separator);

        for split in splits {
            let mut context = context.clone();
            context.insert("split", split);

            for action in &self.actions {
                action.execute(archetect, archetype, destination, answers, &mut context)?;
            }
        }

        Ok(())
    }
}