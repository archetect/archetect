use crate::errors::AnswerParseError;
use pest::iterators::Pair;
use pest::Parser;

#[derive(Parser)]
#[grammar = "config/answer_grammar.pest"]
struct AnswerParser;

pub fn parse_answer_pair(input: &str) -> Result<(String, String), AnswerParseError> {
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
    use rhai::Dynamic;

    #[test]
    fn test_parse_rhai_list() {
        let engine = rhai::Engine::new();
        let value: Dynamic = engine.eval("[\"one\",\"two\",\"three\"]").unwrap();
        assert!(value.is_array());
    }

    #[test]
    fn test_parse_rhai_map() {
        let engine = rhai::Engine::new();
        let value: Dynamic = engine.eval("#{ \"first_name\": \"Jimmie\" }").unwrap();
        assert!(value.is_map());
    }

    #[test]
    fn test_parse_rhai_string() {
        // let engine = rhai::Engine::new();
        // TODO: Fix
        // let value: Dynamic = engine.eval("Value").unwrap();
        // assert!(value.is_string());
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
