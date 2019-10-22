use std::collections::HashMap;
use std::io::Write;

use pest::error::Error as PestError;
use pest::iterators::Pair;
use pest::Parser;
use std::convert::TryFrom;
use std::io;

#[derive(Parser)]
#[grammar = "parser/grammar.pest"]
pub struct ArchetectParser;

#[derive(Debug, Fail)]
pub enum TemplateError {
    #[fail(display = "Rule Error")]
    PestError(PestError<Rule>),
}

impl From<PestError<Rule>> for TemplateError {
    fn from(error: PestError<Rule>) -> Self {
        TemplateError::PestError(error)
    }
}

#[derive(Debug, Fail)]
pub enum RenderError {
    #[fail(display = "Rule Error")]
    IOError(std::io::Error),
}

impl From<io::Error> for RenderError {
    fn from(err: io::Error) -> Self {
        RenderError::IOError(err)
    }
}

#[derive(Debug)]
pub struct Template<'template> {
    nodes: Vec<Node<'template>>,
}

impl<'template> TryFrom<&'template str> for Template<'template> {
    type Error = TemplateError;

    fn try_from(value: &str) -> Result<Template, TemplateError> {
        parse(value)
    }
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq)]
pub enum Value<'a> {
    String(&'a str),
    Int32(i32),
    List(Vec<Value<'a>>),
}

pub struct Context<'a> {
    parent: Option<&'a Context<'a>>,
    variables: HashMap<String, String>,
}

impl<'a> Context<'a> {
    pub fn insert<K: Into<String>, V: Into<String>>(&mut self, key: K, value: V) -> &mut Context<'a> {
        self.variables.insert(key.into(), value.into());
        self
    }

    pub fn scoped(&self) -> Context {
        Context {
            parent: Some(self),
            variables: HashMap::new(),
        }
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        match self.variables.get(key) {
            Some(value) => Some(value.as_str()),
            None => match self.parent {
                Some(parent) => parent.variables.get(key).map(|v| v.as_str()),
                None => None,
            },
        }
    }
}

impl<'a> Default for Context<'a> {
    fn default() -> Self {
        Context {
            parent: None,
            variables: HashMap::new(),
        }
    }
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq)]
pub struct VariableBlock<'template> {
    name: &'template str,
    escape: bool,
    filters: Vec<FilterInfo<'template>>,
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq)]
pub struct FilterInfo<'template> {
    name: &'template str,
    args: Vec<Value<'template>>,
}

pub struct Renderer;

impl Renderer {
    pub fn render<DEST: Write>(
        template: &Template,
        context: &Context,
        destination: &mut DEST,
    ) -> Result<(), RenderError> {
        for node in &template.nodes {
            match node {
                Node::Text(text) => {
                    destination.write(text.as_bytes())?;
                }
                Node::VariableBlock(variable) => {
                    let value = context.get(variable.name).unwrap_or_else(|| "");
                    destination.write(value.as_bytes())?;
                }
            }
        }
        destination.flush()?;
        Ok(())
    }
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq)]
pub enum Node<'template> {
    Text(&'template str),
    VariableBlock(VariableBlock<'template>),
}

pub fn parse(template: &str) -> Result<Template, TemplateError> {
    let mut pairs = ArchetectParser::parse(Rule::template, template)?;
    let mut nodes = Vec::new();
    for pair in pairs.next().unwrap().into_inner() {
        match pair.as_rule() {
            Rule::node => parse_nodes(pair, &mut nodes),
            Rule::EOI => (),
            _ => eprintln!("Unhandled: {:?}", pair),
        }
    }
    Ok(Template { nodes })
}

fn parse_nodes<'template>(pairs: Pair<'template, Rule>, nodes: &mut Vec<Node<'template>>) {
    for pair in pairs.into_inner() {
        match pair.as_rule() {
            Rule::text => nodes.push(parse_text(pair)),
            Rule::variable_block | Rule::raw_variable_block => nodes.push(parse_variable_block(pair)),
            _ => eprintln!("Unhandled: {:?}", pair),
        }
    }
}

fn parse_text(pair: Pair<Rule>) -> Node {
    assert_eq!(pair.as_rule(), Rule::text);
    Node::Text(pair.as_str())
}

fn parse_value(pair: Pair<Rule>) -> Value {
    assert_eq!(pair.as_rule(), Rule::value);
    let pair = pair.into_inner().next().unwrap();
    match pair.as_rule() {
        Rule::string => Value::String(parse_string(pair)),
        Rule::i32 => Value::Int32(pair.as_str().parse().unwrap()),
        _ => unreachable!(),
    }
}

fn parse_filter(pair: Pair<Rule>) -> FilterInfo {
    assert_eq!(pair.as_rule(), Rule::filter);
    let mut iter = pair.into_inner();

    let name = iter.next().unwrap().as_str();

    let args = iter.next().map_or_else(|| vec![], |p| parse_filter_args(p));

    FilterInfo { name, args }
}

fn parse_filter_args(pair: Pair<Rule>) -> Vec<Value> {
    assert_eq!(pair.as_rule(), Rule::filter_args);
    pair.into_inner().map(|p| parse_filter_arg(p)).collect()
}

fn parse_filter_arg(pair: Pair<Rule>) -> Value {
    assert_eq!(pair.as_rule(), Rule::filter_arg);
    parse_value(pair.into_inner().next().unwrap())
}

fn parse_string(pair: Pair<Rule>) -> &str {
    assert_eq!(pair.as_rule(), Rule::string);
    pair.into_inner().next().unwrap().as_str()
}

fn parse_variable_block(pair: Pair<Rule>) -> Node {
    let rule = pair.as_rule();
    assert!(rule == Rule::variable_block || rule == Rule::raw_variable_block);

    let escape = rule == Rule::variable_block;
    let mut iter = pair.into_inner();
    let name = iter.next().unwrap().as_str();

    let filters = iter.map(|p| parse_filter(p)).collect();

    Node::VariableBlock(VariableBlock { name, escape, filters })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template() {
        println!(
            "{:?}",
            ArchetectParser::parse(Rule::template, "Hello, {{ fname }}!")
                .unwrap()
                .next()
                .unwrap()
        );
    }

    #[test]
    fn test_parse_string() {
        assert_eq!(parse_string(parse_pair(Rule::string, "\'Howdy!\'")), "Howdy!");
        assert_eq!(parse_string(parse_pair(Rule::string, "\"Howdy!\"")), "Howdy!");
    }

    #[test]
    fn test_parse_text() {
        assert_eq!(
            parse_text(parse_pair(Rule::text, "Random Text")),
            Node::Text("Random Text")
        );
    }

    #[test]
    fn test_parse_variable_block() {
        assert_eq!(
            parse_variable_block(parse_pair(Rule::variable_block, "{{ subject }} ")),
            Node::VariableBlock(VariableBlock {
                name: "subject",
                escape: true,
                filters: vec![]
            })
        );

        assert_eq!(
            parse_variable_block(parse_pair(Rule::variable_block, "{{ subject | upper }} ")),
            Node::VariableBlock(VariableBlock {
                name: "subject",
                escape: true,
                filters: vec![FilterInfo {
                    name: "upper",
                    args: vec![]
                }]
            })
        );

        assert_eq!(
            parse_variable_block(parse_pair(Rule::variable_block, "{{ subject | upper | trim }} ")),
            Node::VariableBlock(VariableBlock {
                name: "subject",
                escape: true,
                filters: vec![
                    FilterInfo {
                        name: "upper",
                        args: vec![]
                    },
                    FilterInfo {
                        name: "trim",
                        args: vec![]
                    }
                ]
            })
        );

        assert_eq!(
            parse_variable_block(parse_pair(Rule::raw_variable_block, "{{{ message | lower | trim }}} ")),
            Node::VariableBlock(VariableBlock {
                name: "message",
                escape: false,
                filters: vec![
                    FilterInfo {
                        name: "lower",
                        args: vec![]
                    },
                    FilterInfo {
                        name: "trim",
                        args: vec![]
                    }
                ]
            })
        );

        assert_eq!(
            parse_variable_block(parse_pair(
                Rule::variable_block,
                "{{ subject | upper | elide( 99 , \"..\") }} "
            )),
            Node::VariableBlock(VariableBlock {
                name: "subject",
                escape: true,
                filters: vec![
                    FilterInfo {
                        name: "upper",
                        args: vec![]
                    },
                    FilterInfo {
                        name: "elide",
                        args: vec![Value::Int32(99), Value::String("..")]
                    }
                ]
            })
        );
    }

    #[test]
    fn test_parse_value() {
        assert_eq!(parse_value(parse_pair(Rule::value, "12345")), Value::Int32(12345));
        assert_eq!(
            parse_value(parse_pair(Rule::value, "\"String\"")),
            Value::String("String")
        );
    }

    #[test]
    fn test_parse_filter_arg() {
        assert_eq!(
            parse_filter_arg(parse_pair(Rule::filter_arg, "1234")),
            Value::Int32(1234)
        );
    }

    #[test]
    fn test_parse_filter_args() {
        assert_eq!(
            parse_filter_args(parse_pair(Rule::filter_args, "(1234, \"1234\")")),
            vec![Value::Int32(1234), Value::String("1234")]
        );

        assert_eq!(parse_filter_args(parse_pair(Rule::filter_args, "()")), vec![]);
    }

    #[test]
    fn test_parse_filter() {
        assert_eq!(
            parse_filter(parse_pair(Rule::filter, "| join(', ')")),
            FilterInfo {
                name: "join",
                args: vec![Value::String(", ")]
            }
        );

        assert_eq!(
            parse_filter(parse_pair(Rule::filter, "| elide(100, \"...\")")),
            FilterInfo {
                name: "elide",
                args: vec![Value::Int32(100), Value::String("...")]
            }
        );

        assert_eq!(
            parse_filter(parse_pair(Rule::filter, "| join()")),
            FilterInfo {
                name: "join",
                args: vec![]
            }
        );
    }

    #[test]
    fn test_render() -> Result<(), failure::Error> {
        let mut context = Context::default();
        context.insert("subject", "World").insert("adjective", "small");

        let template = Template::try_from("Hello, {{ adjective }} {{ subject }}!")?;
        let mut buffer: Vec<u8> = Vec::new();
        Renderer::render(&template, &context, &mut buffer)?;
        println!("{}", String::from_utf8(buffer).unwrap());

        let mut child = context.scoped();
        child.insert("adjective", "big");
        let mut buffer: Vec<u8> = Vec::new();
        Renderer::render(&template, &child, &mut buffer)?;
        println!("{}", String::from_utf8(buffer).unwrap());

        Ok(())
    }

    #[test]
    #[ignore]
    fn test_context_scope() {
        let mut parent = Context::default();
        parent.insert("message", "Hello");

        let mut child = parent.scoped();
        child.insert("message", "Howdy!");
        child.insert("subject", "World");

        assert_eq!(child.get("message"), Some("Howdy!"));
        assert_eq!(child.get("subject"), Some("World"));
    }

    fn parse_pair(rule: Rule, input: &str) -> Pair<Rule> {
        ArchetectParser::parse(rule, input).unwrap().next().unwrap()
    }
}
