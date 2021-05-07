#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RuleConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    patterns: Vec<Pattern>,
    #[serde(skip_serializing_if = "Option::is_none")]
    filter: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    action: Option<RuleAction>,
}

impl RuleConfig {
    pub fn new() -> RuleConfig {
        RuleConfig {
            description: None,
            patterns: vec![],
            filter: None,
            action: None,
        }
    }

    pub fn with_pattern(mut self, pattern: Pattern) -> RuleConfig {
        self.add_pattern(pattern);
        self
    }

    pub fn add_pattern(&mut self, pattern: Pattern) {
        self.patterns.push(pattern);
    }

    pub fn with_action(mut self, action: RuleAction) -> RuleConfig {
        self.set_action(Some(action));
        self
    }

    pub fn set_action(&mut self, action: Option<RuleAction>) {
        self.action = action;
    }

    pub fn action(&self) -> RuleAction {
        self.action.as_ref().map(|a| a.clone()).unwrap_or_default()
    }

    pub fn patterns(&self) -> &[Pattern] {
        self.patterns.as_slice()
    }

    pub fn add_description(&mut self, description: &str) {
        self.description = Some(description.to_owned());
    }

    pub fn with_description(mut self, description: &str) -> RuleConfig {
        self.add_description(description);
        self
    }

    pub fn description(&self) -> Option<&str> {
        self.description.as_ref().map(|d| d.as_str())
    }

    pub fn filter(&self) -> Option<bool> {
        self.filter
    }
}

#[derive(Debug, Serialize, Deserialize, PartialOrd, PartialEq, Clone)]
pub enum Pattern {
    #[serde(rename = "glob")]
    GLOB(String),
    #[serde(rename = "regex")]
    REGEX(String),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum RuleAction {
    COPY,
    RENDER,
    SKIP,
}

impl Default for RuleAction {
    fn default() -> Self {
        RuleAction::RENDER
    }
}

#[cfg(test)]
mod tests {
    use crate::config::rule::{Pattern, RuleConfig};
    use crate::config::RuleAction;

    #[test]
    fn test_serialize_rule_config() {
        let result = serde_yaml::to_string(
            &RuleConfig::new()
                .with_pattern(Pattern::GLOB("*.jpg".to_owned()))
                .with_pattern(Pattern::GLOB("*.gif".to_owned()))
                .with_action(RuleAction::COPY),
        )
        .unwrap();
        println!("{}", result);
    }

    #[test]
    fn test_serialize_vec_rule_config() {
        let rules = vec![
            RuleConfig::new()
                .with_pattern(Pattern::GLOB("*.jpg".to_owned()))
                .with_pattern(Pattern::GLOB("*.gif".to_owned()))
                .with_action(RuleAction::COPY),
            RuleConfig::new()
                .with_pattern(Pattern::REGEX("^(.*)*.java".to_owned()))
                .with_action(RuleAction::RENDER),
        ];

        let result = serde_yaml::to_string(&rules).unwrap();
        println!("{}", result);
    }
}
