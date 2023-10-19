use archetect_core::errors::{AnswerFileError, ArchetectError};
use camino::Utf8Path;
use rhai::{Dynamic, Engine, Map};
use std::fs;

pub fn read_answers<P: AsRef<Utf8Path>>(path: P) -> Result<Map, ArchetectError> {
    let path = path.as_ref();
    if !path.is_file() {
        return Err(ArchetectError::AnswerConfigError {
            path: path.to_string(),
            source: AnswerFileError::MissingError,
        });
    }
    match path.extension() {
        Some("yml") => read_yaml_answers(&path),
        Some("yaml") => read_yaml_answers(&path),
        Some("json") => read_json_answers(&path),
        Some("rhai") => read_rhai_answers(&path),
        Some(_extension) => Err(ArchetectError::AnswerConfigError {
            path: path.to_string(),
            source: AnswerFileError::InvalidFileType,
        }),
        None => Err(ArchetectError::AnswerConfigError {
            path: path.to_string(),
            source: AnswerFileError::InvalidFileType,
        }),
    }
}

fn read_yaml_answers(path: &Utf8Path) -> Result<Map, ArchetectError> {
    let contents = fs::read_to_string(path)?;
    let result: Dynamic = serde_yaml::from_str(&contents).unwrap();

    if let Some(map) = result.try_cast::<Map>() {
        Ok(map)
    } else {
        Err(ArchetectError::AnswerConfigError {
            path: path.to_string(),
            source: AnswerFileError::InvalidYamlAnswerFileStructure,
        })
    }
}

fn read_json_answers(path: &Utf8Path) -> Result<Map, ArchetectError> {
    let contents = fs::read_to_string(path)?;
    let result: Dynamic = serde_json::from_str(&contents).unwrap();

    if let Some(map) = result.try_cast::<Map>() {
        Ok(map)
    } else {
        Err(ArchetectError::AnswerConfigError {
            path: path.to_string(),
            source: AnswerFileError::InvalidJsonAnswerFileStructure,
        })
    }
}

fn read_rhai_answers(path: &Utf8Path) -> Result<Map, ArchetectError> {
    let contents = fs::read_to_string(path)?;
    let result: Dynamic = Engine::new().eval::<Dynamic>(&contents).unwrap();

    if let Some(map) = result.try_cast::<Map>() {
        Ok(map)
    } else {
        Err(ArchetectError::AnswerConfigError {
            path: path.to_string(),
            source: AnswerFileError::InvalidRhaiAnswerFileStructure,
        })
    }
}

use pest::iterators::Pair;
use pest::Parser;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "answers/answer_grammar.pest"]
struct AnswerParser;

pub fn parse_answer_pair(input: &str) -> Result<(String, String), anyhow::Error> {
    let mut pairs = AnswerParser::parse(Rule::answer, input)?;

    let mut iter = pairs.next().unwrap().into_inner();
    let identifier_pair = iter.next().unwrap();
    let value_pair = iter.next().unwrap();
    Ok((parse_identifier(identifier_pair), parse_value(value_pair)))
}

fn parse_identifier(pair: Pair<Rule>) -> String {
    assert_eq!(pair.as_rule(), Rule::identifier);
    pair.as_str().to_owned()
}

fn parse_value(pair: Pair<Rule>) -> String {
    assert_eq!(pair.as_rule(), Rule::string);
    pair.into_inner().next().unwrap().as_str().to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

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
