use std::collections::HashMap;
use std::io::Write;

use pest::error::Error as PestError;
use pest::iterators::Pair;
use pest::Parser;
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

pub struct Context<'a> {
    parent: Option<&'a Context<'a>>,
    variables: HashMap<String, String>,
}

impl<'a> Context<'a> {
    pub fn insert<K: Into<String>, V: Into<String>>(
        &mut self,
        key: K,
        value: V,
    ) -> &mut Context<'a> {
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

pub struct Renderer;

impl Renderer {
    pub fn render<SYNC: Write>(
        template: &Template,
        context: &Context,
        destination: &mut SYNC,
    ) -> Result<(), RenderError> {
        for node in &template.nodes {
            match node {
                Node::Text(text) => {
                    destination.write(text.as_bytes())?;
                }
                Node::VariableBlock(variable) => {
                    let value = context.get(variable).unwrap_or_else(|| "");
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
    VariableBlock(&'template str),
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
            Rule::text => nodes.push(Node::Text(pair.as_str())),
            Rule::variable_block => parse_variable_block(pair, nodes),
            _ => eprintln!("Unhandled: {:?}", pair),
        }
    }
}

fn parse_variable_block<'template>(pairs: Pair<'template, Rule>, nodes: &mut Vec<Node<'template>>) {
    nodes.push(Node::VariableBlock(
        pairs.into_inner().next().unwrap().as_str(),
    ))
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
    fn test_parse() {
        println!("{:?}", parse(r#"Hello, {{subject | title | join(",") }}!"#));
    }

    #[test]
    fn test_render() -> Result<(), failure::Error> {
        let mut context = Context::default();
        context
            .insert("subject", "World")
            .insert("adjective", "small");

        let template = parse("Hello, {{ adjective }} {{ subject }}!")?;
        let mut buffer: Vec<u8> = Vec::new();
        Renderer::render(&template, &context, &mut buffer)?;
        println!("{}", String::from_utf8(buffer).unwrap());

        let mut child = context.scoped();
        child.insert("adjective", "big");
        let mut buffer: Vec<u8> = Vec::new();
        Renderer::render(&template, &child, &mut buffer);
        println!("{}", String::from_utf8(buffer).unwrap());

        Ok(())
    }

    #[test]
    fn test_context_scope() {
        let mut parent = Context::default();
        parent.insert("message", "Hello");

        let mut child = parent.scoped();
        child.insert("message", "Howdy!");
        child.insert("subject", "World");

        assert_eq!(child.get("message"), Some("Howdy!"));
        assert_eq!(child.get("subject"), Some("World"));
    }
}
