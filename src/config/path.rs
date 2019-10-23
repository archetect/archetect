#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PathRuleConfig {
    description: Option<String>,
    patterns: Vec<String>,
    #[serde(rename = "type")]
    pattern_type: PatternType,
    filter: Option<bool>,
}

impl PathRuleConfig {
    pub fn new(pattern_type: PatternType) -> PathRuleConfig {
        PathRuleConfig {
            description: None,
            pattern_type,
            patterns: vec![],
            filter: None,
        }
    }

    pub fn with_pattern(mut self, pattern: &str) -> PathRuleConfig {
        self.add_pattern(pattern);
        self
    }

    pub fn add_pattern(&mut self, pattern: &str) {
        self.patterns.push(pattern.to_owned());
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

    pub fn with_description(mut self, description: &str) -> PathRuleConfig {
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

#[cfg(test)]
mod tests {
    use crate::config::path::{PathRuleConfig, PatternType};

    #[test]
    fn test_serialize_path_config() {
        let result = toml::ser::to_string(
            &PathRuleConfig::new(PatternType::GLOB)
                .with_pattern("*.jpg")
                .with_pattern("*.gif"),
        )
        .unwrap();
        println!("{}", result);
    }
}
