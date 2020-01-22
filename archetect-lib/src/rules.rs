use std::path::Path;

use linked_hash_map::LinkedHashMap;
use log::{trace};

use crate::config::{Pattern, RuleAction, RuleConfig};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RulesContext {
    overwrite: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    path_rules: Option<LinkedHashMap<String, RuleConfig>>,
    break_triggered: bool,
}

impl RulesContext {
    pub fn new() -> RulesContext {
        RulesContext {
            overwrite: false,
            path_rules: None,
            break_triggered: false,
        }
    }

    pub fn set_overwrite(&mut self, overwrite: bool) {
        self.overwrite = overwrite;
    }

    pub fn overwrite(&self) -> bool {
        self.overwrite
    }

    pub fn path_rules_mut(&mut self) -> Option<&mut LinkedHashMap<String, RuleConfig>> {
        self.path_rules.as_mut()
    }

    pub fn path_rules(&self) -> Option<&LinkedHashMap<String, RuleConfig>> {
        self.path_rules.as_ref()
    }

    pub fn break_triggered(&self) -> bool {
        self.break_triggered
    }

    pub fn set_break_triggered(&mut self, break_triggered: bool) {
        self.break_triggered = break_triggered;
    }

    pub fn insert_path_rules(&mut self, insert: &LinkedHashMap<String, RuleConfig>) {
        let mut results = insert.clone();
        let path_rules = self.path_rules.get_or_insert_with(|| LinkedHashMap::new());
        for (name, options) in path_rules {
            results.insert(name.to_owned(), options.clone());
        }
        self.path_rules = Some(results);
    }

    pub fn append_path_rules(&mut self, append: &LinkedHashMap<String, RuleConfig>) {
        let path_rules = self.path_rules.get_or_insert_with(|| LinkedHashMap::new());
        for (name, options) in append {
            path_rules.insert(name.to_owned(), options.clone());
        }
    }

    pub fn get_source_action<P: AsRef<Path>>(&self, path: P) -> RuleAction {
        if let Some(path_rules) = self.path_rules() {
            let path = path.as_ref();
            for (name, path_rule) in path_rules {
                for pattern in path_rule.patterns() {
                    match pattern {
                        Pattern::GLOB(pattern) => {
                            let matcher = glob::Pattern::new(pattern).unwrap();
                            if matcher.matches_path(&path) {
                                trace!("Source Rule [{}: {:?} {:?}] matched '{}'",
                                       name, &path_rule.action(), pattern, path.display());
                                return path_rule.action().clone();
                            }
                        }
                        _ => unimplemented!()
                    }
                }
            }
        }
        RuleAction::RENDER
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum WriteRule {
    #[serde(rename = "IF_MISSING")]
    IsMissing,
    #[serde(rename = "ALWAYS")]
    Always,
}