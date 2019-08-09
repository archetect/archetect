use std::collections::HashMap;
use std::path::PathBuf;
use crate::ArchetypeError;
use std::{fs, fmt};
use std::str::FromStr;
use std::fmt::{Display, Formatter};

use pest::iterators::Pair;
use pest::error::Error as PestError;
use pest::Parser;

#[derive(Debug, Deserialize, Serialize)]
pub struct AnswerConfig {
    pub(crate) answers: HashMap<String, String>,
}

impl AnswerConfig {
    pub fn load<P: Into<PathBuf>>(path: P) -> Result<AnswerConfig, ArchetypeError> {
        let path = path.into();
        if path.is_dir() {
            let dot_answers = path.clone().join(".answers.toml");
            if dot_answers.exists() {
                let config = fs::read_to_string(dot_answers).unwrap();
                let config = toml::de::from_str::<AnswerConfig>(&config).unwrap();
                return Ok(config);
            }

            let answers = path.clone().join("answers.toml");
            if answers.exists() {
                let config = fs::read_to_string(answers).unwrap();
                let config = toml::de::from_str::<AnswerConfig>(&config).unwrap();
                return Ok(config);
            }
        } else {
            let config = fs::read_to_string(path).unwrap();
            let config = toml::de::from_str::<AnswerConfig>(&config).unwrap();
            return Ok(config);
        }

        Err(ArchetypeError::InvalidAnswersConfig)
    }

    pub fn add_answer<K: Into<String>, V: Into<String>>(&mut self, key: K, value: V) {
        self.answers.insert(key.into(), value.into());
    }

    pub fn answers(&self) -> &HashMap<String, String> {
        &self.answers
    }
}

impl Default for AnswerConfig {
    fn default() -> Self {
        AnswerConfig {
            answers: HashMap::new(),
        }
    }
}

impl FromStr for AnswerConfig {
    type Err = ArchetypeError;

    fn from_str(config: &str) -> Result<Self, Self::Err> {
        toml::de::from_str::<AnswerConfig>(config).map_err(|_| ArchetypeError::ArchetypeInvalid)
    }
}

impl Display for AnswerConfig {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match toml::ser::to_string_pretty(self) {
            Ok(config) => write!(f, "{}", config),
            Err(_) => Err(fmt::Error),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AnswerEntry {
    variable: String,
    answer: String,
}

impl AnswerEntry {
    pub fn new<V: Into<String>, A: Into<String>>(variable: V, answer: A) -> AnswerEntry {
        AnswerEntry {
            variable: variable.into(),
            answer: answer.into(),
        }
    }
}

#[derive(Parser)]
#[grammar = "config/answer_grammar.pest"]
struct AnswerParser;

#[derive(Debug, Fail, PartialEq)]
pub enum AnswerParseError {
    #[fail(display = "Answer Rule Error")]
    PestError(PestError<Rule>),
}

impl From<PestError<Rule>> for AnswerParseError {
    fn from(error: PestError<Rule>) -> Self {
        AnswerParseError::PestError(error)
    }
}

fn parse(source: &str) -> Result<Answer, AnswerParseError> {
    let mut pairs = AnswerParser::parse(Rule::answer, source)?;
    Ok(parse_answer(pairs.next().unwrap()))
}

fn parse_answer(pair: Pair<Rule>) -> Answer {
    assert_eq!(pair.as_rule(), Rule::answer);
    let mut iter = pair.into_inner();
    let identifier_pair = iter.next().unwrap();
    let value_pair = iter.next().unwrap();
    Answer { identifier: parse_identifier(identifier_pair), value: parse_value(value_pair) }
}

fn parse_identifier(pair: Pair<Rule>) -> String {
    assert_eq!(pair.as_rule(), Rule::identifier);
    pair.as_str().to_owned()
}

fn parse_value(pair: Pair<Rule>) -> String {
    assert_eq!(pair.as_rule(), Rule::string);
    pair.into_inner().next().unwrap().as_str().to_owned()
}

#[derive(PartialOrd, PartialEq, Debug)]
pub struct Answer {
    identifier: String,
    value: String,
}

impl Answer {
    pub fn new<I: Into<String>, V: Into<String>>(identifier: I, value: V) -> Answer {
        Answer { identifier: identifier.into(), value: value.into() }
    }

    pub fn parse(input: &str) -> Result<Answer, AnswerParseError> {
        parse(input)
    }

    pub fn identifier(&self) -> &str {
        &self.identifier
    }

    pub fn value(&self) -> &str {
        &self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_success() {
        assert_eq!(
            parse("key=value"),
            Ok(Answer { identifier: "key".to_string(), value: "value".to_string() })
        );

        assert_eq!(
            parse("key = value"),
            Ok(Answer::new("key", "value"))
        );

        assert_eq!(
            parse("key = value set"),
            Ok(Answer { identifier: "key".to_string(), value: "value set".to_string() })
        );

        assert_eq!(
            parse("key='value'"),
            Ok(Answer { identifier: "key".to_string(), value: "value".to_string() })
        );

        assert_eq!(
            parse("key='value set'"),
            Ok(Answer { identifier: "key".to_string(), value: "value set".to_string() })
        );

        assert_eq!(
            parse("key = 'value'"),
            Ok(Answer { identifier: "key".to_string(), value: "value".to_string() })
        );

        assert_eq!(
            parse("key=\"value\""),
            Ok(Answer { identifier: "key".to_string(), value: "value".to_string() })
        );

        assert_eq!(
            parse("key=\"value set\""),
            Ok(Answer { identifier: "key".to_string(), value: "value set".to_string() })
        );

        assert_eq!(
            parse("key = \"value\""),
            Ok(Answer { identifier: "key".to_string(), value: "value".to_string() })
        );

        assert_eq!(
            parse("key ="),
            Ok(Answer { identifier: "key".to_string(), value: "".to_string() })
        );

        assert_eq!(
            parse("key =''"),
            Ok(Answer { identifier: "key".to_string(), value: "".to_string() })
        );

        assert_eq!(
            parse(" key =\"\""),
            Ok(Answer { identifier: "key".to_string(), value: "".to_string() })
        );
    }

    #[test]
    fn test_parse_fail() {
        match parse("key") {
            Err(AnswerParseError::PestError(_)) => (),
            _ => panic!("Error expected"),
        }
    }

    #[test]
    fn test_parse_answer() {
        assert_eq!(
            parse_answer(AnswerParser::parse(Rule::answer, "key=value").unwrap().next().unwrap()),
            Answer { identifier: "key".to_string(), value: "value".to_string() }
        );

        assert_eq!(
            parse_answer(AnswerParser::parse(Rule::answer, "key='value'").unwrap().next().unwrap()),
            Answer { identifier: "key".to_string(), value: "value".to_string() }
        );

        assert_eq!(
            parse_answer(AnswerParser::parse(Rule::answer, "key=\"value\"").unwrap().next().unwrap()),
            Answer { identifier: "key".to_string(), value: "value".to_string() }
        );
    }

    #[test]
    fn test_parse_identifier() {
        assert_eq!(
            parse_identifier(AnswerParser::parse(Rule::identifier, "key").unwrap().next().unwrap()),
            "key"
        );
    }

    #[test]
    fn test_parse_value() {
        assert_eq!(
            parse_value(AnswerParser::parse(Rule::string, "value").unwrap().next().unwrap()),
            "value"
        );

        assert_eq!(
            parse_value(AnswerParser::parse(Rule::string, "\"value\"").unwrap().next().unwrap()),
            "value"
        );

        assert_eq!(
            parse_value(AnswerParser::parse(Rule::string, "'value'").unwrap().next().unwrap()),
            "value"
        );
    }
}