use crate::config::{RuleConfig, VariableInfo};
use crate::actions::Action;
use std::path::Path;
use crate::{Archetect, Archetype, ArchetectError};
use crate::rules::RulesContext;
use linked_hash_map::LinkedHashMap;
use crate::template_engine::Context;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum RuleType {
    #[serde(rename = "destination")]
    DestinationRules(DestinationOptions),
    #[serde(rename = "source")]
    SourceRules(LinkedHashMap<String, RuleConfig>),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DestinationOptions {
    overwrite: bool,
}

impl Action for RuleType {
    fn execute<D: AsRef<Path>>(&self,
                               _archetect: &Archetect,
                               _archetype: &Archetype,
                               _destination: D,
                               rules_context: &mut RulesContext,
                               _answers: &LinkedHashMap<String, VariableInfo>,
                               _context: &mut Context
    ) -> Result<(), ArchetectError> {
        match self {
            RuleType::SourceRules(rules) => {
                rules_context.insert_path_rules(rules);
            }
            RuleType::DestinationRules(options) => {
                rules_context.set_overwrite(options.overwrite);
            }
        }
        Ok(())
    }
}