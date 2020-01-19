
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RuleConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    patterns: Vec<String>,
    #[serde(rename = "type")]
    pattern_type: PatternType,
    #[serde(skip_serializing_if = "Option::is_none")]
    filter: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    action: Option<RuleAction>,
}

impl RuleConfig {
    pub fn new(pattern_type: PatternType) -> RuleConfig {
        RuleConfig {
            description: None,
            pattern_type,
            patterns: vec![],
            filter: None,
            action: None,
        }
    }

    pub fn with_pattern(mut self, pattern: &str) -> RuleConfig {
        self.add_pattern(pattern);
        self
    }

    pub fn add_pattern(&mut self, pattern: &str) {
        self.patterns.push(pattern.to_owned());
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

    pub fn patterns(&self) -> &[String] {
        self.patterns.as_slice()
    }

    pub fn pattern_type(&self) -> &PatternType {
        &self.pattern_type
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
pub enum PatternType {
    GLOB,
    REGEX,
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
    use crate::config::rule::{RuleConfig, PatternType};
    use crate::config::RuleAction;

    #[test]
    fn test_serialize_rule_config() {
        let result = serde_yaml::to_string(
            &RuleConfig::new(PatternType::GLOB)
                .with_pattern("*.jpg")
                .with_pattern("*.gif")
                .with_action(RuleAction::COPY)
            ,
        )
        .unwrap();
        println!("{}", result);
    }

    #[test]
    fn test_serialize_vec_rule_config() {
        let rules = vec![
            RuleConfig::new(PatternType::GLOB)
                .with_pattern("*.jpg")
                .with_pattern("*.gif")
                .with_action(RuleAction::COPY),
            RuleConfig::new(PatternType::REGEX)
                .with_pattern("^(.*)*.java")
                .with_action(RuleAction::RENDER)
        ];

        let result = serde_yaml::to_string(&rules).unwrap();
        println!("{}", result);
    }
}
