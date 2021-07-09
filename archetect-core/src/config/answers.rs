use std::fs;
use std::path::PathBuf;

use linked_hash_map::LinkedHashMap;
use log::debug;
use pest::error::Error as PestError;
use pest::iterators::Pair;
use pest::Parser;

use crate::config::VariableInfo;

pub type AnswerInfo = VariableInfo;

#[derive(Debug, Deserialize, Serialize)]
pub struct AnswerConfig {
    #[serde(skip_serializing_if = "LinkedHashMap::is_empty")]
    answers: LinkedHashMap<String, AnswerInfo>,
}

#[derive(Debug, PartialEq, thiserror::Error)]
pub enum AnswerConfigError {
    #[error("Error parsing answer config: {0}")]
    ParseError(String),
    #[error("Missing answer config")]
    MissingError,
}

impl From<serde_yaml::Error> for AnswerConfigError {
    fn from(error: serde_yaml::Error) -> Self {
        AnswerConfigError::ParseError(error.to_string())
    }
}

impl From<std::io::Error> for AnswerConfigError {
    fn from(_: std::io::Error) -> Self {
        // TODO: Distinguish between missing and other errors
        AnswerConfigError::MissingError
    }
}

impl AnswerConfig {
    pub fn load<P: Into<PathBuf>>(path: P) -> Result<AnswerConfig, AnswerConfigError> {
        let path = path.into();
        if path.is_dir() {
            let answer_file_names = vec![
                "archetect.yml",
                ".archetect.yml",
                "archetect.yaml",
                ".archetect.yaml",
                ".answers.yaml",
                "answers.yaml",
            ];
            for answer_file_name in answer_file_names {
                let answers = path.join(answer_file_name);
                if answers.exists() {
                    debug!("Reading Archetect config from '{}'", &answers.display());
                    let config = fs::read_to_string(answers)?;
                    let config = serde_yaml::from_str::<AnswerConfig>(&config)?;
                    return Ok(config);
                }
            }
        } else {
            let config = fs::read_to_string(path)?;
            let config = serde_yaml::from_str::<AnswerConfig>(&config)?;
            return Ok(config);
        }

        // TODO: Return Ok(None) instead of error
        Err(AnswerConfigError::MissingError)
    }

    pub fn add_answer(&mut self, identifier: &str, value: &str) {
        self.answers
            .insert(identifier.to_owned(), AnswerInfo::with_value(value).build());
    }

    pub fn with_answer(mut self, identifier: &str, value: &str) -> AnswerConfig {
        self.add_answer(identifier, value);
        self
    }

    pub fn answers(&self) -> &LinkedHashMap<String, AnswerInfo> {
        &self.answers
    }
}

impl Default for AnswerConfig {
    fn default() -> Self {
        AnswerConfig {
            answers: LinkedHashMap::new(),
        }
    }
}

#[derive(Parser)]
#[grammar = "config/answer_grammar.pest"]
struct AnswerParser;

#[derive(Debug, PartialEq)]
pub enum AnswerParseError {
    PestError(PestError<Rule>),
}

impl From<PestError<Rule>> for AnswerParseError {
    fn from(error: PestError<Rule>) -> Self {
        AnswerParseError::PestError(error)
    }
}

fn parse(source: &str) -> Result<(String, AnswerInfo), AnswerParseError> {
    let mut pairs = AnswerParser::parse(Rule::answer, source)?;
    Ok(parse_answer(pairs.next().unwrap()))
}

fn parse_answer(pair: Pair<Rule>) -> (String, AnswerInfo) {
    assert_eq!(pair.as_rule(), Rule::answer);
    let mut iter = pair.into_inner();
    let identifier_pair = iter.next().unwrap();
    let value_pair = iter.next().unwrap();
    (
        parse_identifier(identifier_pair),
        AnswerInfo::with_value(parse_value(value_pair)).build(),
    )
}

fn parse_identifier(pair: Pair<Rule>) -> String {
    assert_eq!(pair.as_rule(), Rule::identifier);
    pair.as_str().to_owned()
}

fn parse_value(pair: Pair<Rule>) -> String {
    assert_eq!(pair.as_rule(), Rule::string);
    pair.into_inner().next().unwrap().as_str().to_owned()
}

impl AnswerInfo {
    pub fn parse(input: &str) -> Result<(String, AnswerInfo), AnswerParseError> {
        parse(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_success() {
        assert_eq!(
            parse("key=value"),
            Ok(("key".to_owned(), AnswerInfo::with_value("value").build()))
        );

        assert_eq!(
            parse("key = value"),
            Ok(("key".to_owned(), AnswerInfo::with_value("value").build()))
        );

        assert_eq!(
            parse("key = value set"),
            Ok(("key".to_owned(), AnswerInfo::with_value("value set").build()))
        );

        assert_eq!(
            parse("key='value'"),
            Ok(("key".to_owned(), AnswerInfo::with_value("value").build()))
        );

        assert_eq!(
            parse("key='value set'"),
            Ok(("key".to_owned(), AnswerInfo::with_value("value set").build()))
        );

        assert_eq!(
            parse("key = 'value'"),
            Ok(("key".to_owned(), AnswerInfo::with_value("value").build()))
        );

        assert_eq!(
            parse("key=\"value\""),
            Ok(("key".to_owned(), AnswerInfo::with_value("value").build()))
        );

        assert_eq!(
            parse("key=\"value set\""),
            Ok(("key".to_owned(), AnswerInfo::with_value("value set").build()))
        );

        assert_eq!(
            parse("key = \"value\""),
            Ok(("key".to_owned(), AnswerInfo::with_value("value").build()))
        );

        assert_eq!(
            parse("key ="),
            Ok(("key".to_owned(), AnswerInfo::with_value("").build()))
        );

        assert_eq!(
            parse("key =''"),
            Ok(("key".to_owned(), AnswerInfo::with_value("").build()))
        );

        assert_eq!(
            parse(" key =\"\""),
            Ok(("key".to_owned(), AnswerInfo::with_value("").build()))
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
            ("key".to_owned(), AnswerInfo::with_value("value").build())
        );

        assert_eq!(
            parse_answer(
                AnswerParser::parse(Rule::answer, "key='value'")
                    .unwrap()
                    .next()
                    .unwrap()
            ),
            ("key".to_owned(), AnswerInfo::with_value("value").build())
        );

        assert_eq!(
            parse_answer(
                AnswerParser::parse(Rule::answer, "key=\"value\"")
                    .unwrap()
                    .next()
                    .unwrap()
            ),
            ("key".to_owned(), AnswerInfo::with_value("value").build())
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

    #[test]
    fn test_serialize_answer_config() {
        let config = AnswerConfig::default()
            .with_answer("name", "Order Service")
            .with_answer("author", "Jane Doe");

        println!("{}", serde_yaml::to_string(&config).unwrap());
    }
}
