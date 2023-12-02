pub enum ValueSource {
    Answer,
    DefaultsWith,
    Value,
}

impl ValueSource {
    pub fn error_header(&self) -> String {
        match self {
            ValueSource::Answer => "Answer Error".to_string(),
            ValueSource::DefaultsWith => "defaults_with Error".to_string(),
            ValueSource::Value => "Value Error".to_string(),
        }
    }

    pub fn description(&self) -> String {
        match self {
            ValueSource::Answer => "an answer".to_string(),
            ValueSource::DefaultsWith => "a defaults_with".to_string(),
            ValueSource::Value => "a value".to_string(),
        }
    }
}
