use crate::archetype::ArchetypeError;

use log::debug;

use std::fmt::{Display, Formatter};
use std::path::PathBuf;
use std::str::FromStr;
use std::{fmt, fs};

use pest::error::Error as PestError;
use pest::iterators::Pair;
use pest::Parser;

#[derive(Debug, Deserialize, Serialize)]
pub struct AnswerConfig {
    #[serde(rename = "answer")]
    answers: Vec<Answer>,
}

#[derive(Debug, PartialEq)]
pub enum AnswerConfigError {
    ParseError(toml::de::Error),
    MissingError,
}

impl From<toml::de::Error> for AnswerConfigError {
    fn from(error: toml::de::Error) -> Self {
        AnswerConfigError::ParseError(error)
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
            let dot_answers = path.clone().join(".answers.toml");
            if dot_answers.exists() {
                debug!(target: "archetect", "Reading answers from '{}'", &dot_answers.display());
                let config = fs::read_to_string(dot_answers)?;
                let config = toml::de::from_str::<AnswerConfig>(&config)?;
                return Ok(config);
            }

            let answers = path.clone().join("answers.toml");
            if answers.exists() {
                debug!(target: "archetect", "Reading answers from '{}'", &dot_answers.display());
                let config = fs::read_to_string(answers)?;
                let config = toml::de::from_str::<AnswerConfig>(&config)?;
                return Ok(config);
            }
        } else {
            let config = fs::read_to_string(path)?;
            let config = toml::de::from_str::<AnswerConfig>(&config)?;
            return Ok(config);
        }

        Err(AnswerConfigError::MissingError)
    }

    pub fn add_answer_pair(&mut self, identifier: &str, value: &str) {
        self.answers.push(Answer::new(identifier, value));
    }

    pub fn add_answer(&mut self, answer: Answer) {
        self.answers.push(answer);
    }

    pub fn with_answer_pair(mut self, identifier: &str, value: &str) -> AnswerConfig {
        self.answers.push(Answer::new(identifier, value));
        self
    }

    pub fn with_answer(mut self, answer: Answer) -> AnswerConfig {
        self.answers.push(answer);
        self
    }

    pub fn answers(&self) -> &Vec<Answer> {
        &self.answers
    }
}

impl Default for AnswerConfig {
    fn default() -> Self {
        AnswerConfig { answers: Vec::new() }
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

fn parse(source: &str) -> Result<Answer, AnswerParseError> {
    let mut pairs = AnswerParser::parse(Rule::answer, source)?;
    Ok(parse_answer(pairs.next().unwrap()))
}

fn parse_answer(pair: Pair<Rule>) -> Answer {
    assert_eq!(pair.as_rule(), Rule::answer);
    let mut iter = pair.into_inner();
    let identifier_pair = iter.next().unwrap();
    let value_pair = iter.next().unwrap();
    Answer::new(parse_identifier(identifier_pair), parse_value(value_pair))
}

fn parse_identifier(pair: Pair<Rule>) -> String {
    assert_eq!(pair.as_rule(), Rule::identifier);
    pair.as_str().to_owned()
}

fn parse_value(pair: Pair<Rule>) -> String {
    assert_eq!(pair.as_rule(), Rule::string);
    pair.into_inner().next().unwrap().as_str().to_owned()
}

#[derive(PartialOrd, PartialEq, Debug, Deserialize, Serialize, Clone)]
pub struct Answer {
    identifier: String,
    value: String,
    prompt: Option<bool>,
}

impl Answer {
    pub fn new<I: Into<String>, V: Into<String>>(identifier: I, value: V) -> Answer {
        Answer {
            identifier: identifier.into(),
            value: value.into(),
            prompt: None,
        }
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

    pub fn prompt(&self) -> Option<bool> {
        self.prompt
    }

    pub fn with_prompt(mut self, prompt: bool) -> Answer {
        self.prompt = Some(prompt);
        self
    }

    pub fn set_prompt(&mut self, prompt: Option<bool>) {
        self.prompt = prompt;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_success() {
        assert_eq!(parse("key=value"), Ok(Answer::new("key", "value")));

        assert_eq!(parse("key = value"), Ok(Answer::new("key", "value")));

        assert_eq!(parse("key = value set"), Ok(Answer::new("key", "value set")));

        assert_eq!(parse("key='value'"), Ok(Answer::new("key", "value")));

        assert_eq!(parse("key='value set'"), Ok(Answer::new("key", "value set")));

        assert_eq!(parse("key = 'value'"), Ok(Answer::new("key", "value")));

        assert_eq!(parse("key=\"value\""), Ok(Answer::new("key", "value")));

        assert_eq!(parse("key=\"value set\""), Ok(Answer::new("key", "value set")));

        assert_eq!(parse("key = \"value\""), Ok(Answer::new("key", "value")));

        assert_eq!(parse("key ="), Ok(Answer::new("key", "")));

        assert_eq!(parse("key =''"), Ok(Answer::new("key", "")));

        assert_eq!(parse(" key =\"\""), Ok(Answer::new("key", "")));
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
            Answer::new("key", "value")
        );

        assert_eq!(
            parse_answer(
                AnswerParser::parse(Rule::answer, "key='value'")
                    .unwrap()
                    .next()
                    .unwrap()
            ),
            Answer::new("key", "value")
        );

        assert_eq!(
            parse_answer(
                AnswerParser::parse(Rule::answer, "key=\"value\"")
                    .unwrap()
                    .next()
                    .unwrap()
            ),
            Answer::new("key", "value")
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
            .with_answer_pair("name", "Order Service")
            .with_answer(Answer::new("author", "Jane Doe").with_prompt(true));

        print!("{}", toml::ser::to_string_pretty(&config).unwrap());
    }
}
