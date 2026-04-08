use archetect_api::{ContextMap, ContextValue};
use archetect_core::errors::{AnswerFileError, ArchetectError};
use camino::Utf8Path;
use std::fs;

pub fn read_answers<P: AsRef<Utf8Path>>(path: P) -> Result<ContextMap, ArchetectError> {
    let path = path.as_ref();
    if !path.is_file() {
        return Err(ArchetectError::AnswerConfigError {
            path: path.to_string(),
            source: AnswerFileError::MissingError,
        });
    }
    match path.extension() {
        Some("yml") => read_yaml_answers(path),
        Some("yaml") => read_yaml_answers(path),
        Some("json") => read_json_answers(path),
        Some("rhai") => read_rhai_answers(path),
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

fn read_yaml_answers(path: &Utf8Path) -> Result<ContextMap, ArchetectError> {
    let contents = fs::read_to_string(path)?;
    let result: ContextMap = serde_yaml::from_str(&contents).map_err(|err| ArchetectError::AnswerConfigError {
        path: path.to_string(),
        source: AnswerFileError::ParseError(err.to_string()),
    })?;
    Ok(result)
}

fn read_json_answers(path: &Utf8Path) -> Result<ContextMap, ArchetectError> {
    let contents = fs::read_to_string(path)?;
    let result: ContextMap = serde_json::from_str(&contents).map_err(|err| ArchetectError::AnswerConfigError {
        path: path.to_string(),
        source: AnswerFileError::ParseError(err.to_string()),
    })?;
    Ok(result)
}

fn read_rhai_answers(path: &Utf8Path) -> Result<ContextMap, ArchetectError> {
    // Parse Rhai answer files via the Rhai engine, then bridge to ContextMap
    let contents = fs::read_to_string(path)?;
    let result: rhai::Dynamic = rhai::Engine::new().eval::<rhai::Dynamic>(&contents).map_err(|err| ArchetectError::AnswerConfigError {
        path: path.to_string(),
        source: AnswerFileError::ParseError(err.to_string()),
    })?;

    if let Some(map) = result.try_cast::<rhai::Map>() {
        Ok(archetect_core::conversions::rhai_map_to_context_map(&map))
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

    let answer_pair = pairs.next().ok_or_else(|| anyhow::anyhow!("No answer pair found"))?;
    let mut iter = answer_pair.into_inner();
    let identifier_pair = iter.next().ok_or_else(|| anyhow::anyhow!("Missing identifier"))?;
    let value_pair = iter.next().ok_or_else(|| anyhow::anyhow!("Missing value"))?;
    Ok((parse_identifier(identifier_pair), parse_value(value_pair)?))
}

fn parse_identifier(pair: Pair<Rule>) -> String {
    pair.as_str().to_owned()
}

fn parse_value(pair: Pair<Rule>) -> Result<String, anyhow::Error> {
    let inner = pair.into_inner().next().ok_or_else(|| anyhow::anyhow!("Missing value content"))?;
    Ok(inner.as_str().to_owned())
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
            parse_value(AnswerParser::parse(Rule::string, "value").unwrap().next().unwrap()).unwrap(),
            "value"
        );

        assert_eq!(
            parse_value(AnswerParser::parse(Rule::string, "\"value\"").unwrap().next().unwrap()).unwrap(),
            "value"
        );

        assert_eq!(
            parse_value(AnswerParser::parse(Rule::string, "'value'").unwrap().next().unwrap()).unwrap(),
            "value"
        );
    }
}
