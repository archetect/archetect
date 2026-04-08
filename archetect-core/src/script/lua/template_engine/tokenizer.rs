use super::error::TemplateCompileError;

#[derive(Debug, Clone, PartialEq)]
pub struct Filter {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    /// Literal text emitted as-is.
    Text(String),
    /// `{{ expr | filter1 | filter2 }}` — expression with optional filter chain.
    Expression {
        expr: String,
        filters: Vec<Filter>,
        trim_left: bool,
        trim_right: bool,
    },
    /// `{% lua_code %}` — raw Lua code block.
    Logic {
        code: String,
        trim_left: bool,
        trim_right: bool,
    },
    /// `{# comment #}` — stripped from output.
    Comment,
}

pub struct Tokenizer;

impl Tokenizer {
    pub fn tokenize(template: &str) -> Result<Vec<Token>, TemplateCompileError> {
        let mut tokens = Vec::new();
        let bytes = template.as_bytes();
        let len = bytes.len();
        let mut pos = 0;
        let mut line = 1;

        while pos < len {
            // Find the next `{` character
            let next_brace = match memchr::memchr(b'{', &bytes[pos..]) {
                Some(offset) => pos + offset,
                None => {
                    // Rest is plain text
                    tokens.push(Token::Text(template[pos..].to_string()));
                    break;
                }
            };

            // Check what follows the `{`
            if next_brace + 1 >= len {
                // `{` at end of input — plain text
                tokens.push(Token::Text(template[pos..].to_string()));
                break;
            }

            let next_char = bytes[next_brace + 1];

            match next_char {
                b'{' => {
                    // `{{` — expression
                    if next_brace > pos {
                        let text = &template[pos..next_brace];
                        line += text.matches('\n').count();
                        tokens.push(Token::Text(text.to_string()));
                    }
                    let start_line = line;
                    let content_start = next_brace + 2;
                    match find_closing(template, content_start, "}}", &mut line) {
                        Some(content_end) => {
                            let raw = template[content_start..content_end].trim();
                            let trim_left = raw.starts_with('-');
                            let trim_right = raw.ends_with('-');
                            let raw = raw.trim_start_matches('-').trim_end_matches('-').trim();
                            if raw.is_empty() {
                                return Err(TemplateCompileError::EmptyExpression { line: start_line });
                            }
                            let (expr, filters) = parse_expression(raw, start_line)?;
                            tokens.push(Token::Expression { expr, filters, trim_left, trim_right });
                            pos = content_end + 2;
                        }
                        None => {
                            return Err(TemplateCompileError::UnterminatedExpression { line: start_line });
                        }
                    }
                }
                b'%' => {
                    // `{%` — logic block
                    if next_brace > pos {
                        let text = &template[pos..next_brace];
                        line += text.matches('\n').count();
                        tokens.push(Token::Text(text.to_string()));
                    }
                    let start_line = line;
                    let content_start = next_brace + 2;
                    match find_closing(template, content_start, "%}", &mut line) {
                        Some(content_end) => {
                            let raw = template[content_start..content_end].trim();
                            let trim_left = raw.starts_with('-');
                            let trim_right = raw.ends_with('-');
                            let raw = raw.trim_start_matches('-').trim_end_matches('-').trim();
                            tokens.push(Token::Logic {
                                code: raw.to_string(),
                                trim_left,
                                trim_right,
                            });
                            pos = content_end + 2;
                        }
                        None => {
                            return Err(TemplateCompileError::UnterminatedLogic { line: start_line });
                        }
                    }
                }
                b'#' => {
                    // `{#` — comment
                    if next_brace > pos {
                        let text = &template[pos..next_brace];
                        line += text.matches('\n').count();
                        tokens.push(Token::Text(text.to_string()));
                    }
                    let start_line = line;
                    let content_start = next_brace + 2;
                    match find_closing(template, content_start, "#}", &mut line) {
                        Some(content_end) => {
                            tokens.push(Token::Comment);
                            pos = content_end + 2;
                        }
                        None => {
                            return Err(TemplateCompileError::UnterminatedComment { line: start_line });
                        }
                    }
                }
                _ => {
                    // Not a delimiter — include `{` as text and continue
                    let text = &template[pos..next_brace + 1];
                    line += text.matches('\n').count();
                    tokens.push(Token::Text(text.to_string()));
                    pos = next_brace + 1;
                }
            }
        }

        Ok(tokens)
    }
}

/// Find the closing delimiter (e.g., `}}`, `%}`, `#}`) starting from `start`.
/// Updates `line` to track newlines within the content.
/// Returns the byte position of the closing delimiter start, or None.
fn find_closing(template: &str, start: usize, delimiter: &str, line: &mut usize) -> Option<usize> {
    let delim_bytes = delimiter.as_bytes();
    let bytes = template.as_bytes();
    let mut pos = start;

    while pos + delim_bytes.len() <= bytes.len() {
        if &bytes[pos..pos + delim_bytes.len()] == delim_bytes {
            return Some(pos);
        }
        if bytes[pos] == b'\n' {
            *line += 1;
        }
        pos += 1;
    }

    None
}

/// Parse an expression string into the base expression and filter chain.
/// Handles `expr | filter1 | filter2`.
fn parse_expression(raw: &str, line: usize) -> Result<(String, Vec<Filter>), TemplateCompileError> {
    // Split on `|` but respect nested parentheses, brackets, and strings
    let parts = split_filters(raw);

    let expr = parts[0].trim().to_string();
    if expr.is_empty() {
        return Err(TemplateCompileError::EmptyExpression { line });
    }

    let mut filters = Vec::new();
    for part in &parts[1..] {
        let name = part.trim().to_string();
        if name.is_empty() {
            return Err(TemplateCompileError::InvalidFilter {
                line,
                detail: "empty filter name".to_string(),
            });
        }
        filters.push(Filter { name });
    }

    Ok((expr, filters))
}

/// Split on `|` while respecting parentheses, brackets, and string literals.
fn split_filters(s: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth = 0i32;
    let mut in_string = false;
    let mut string_char = ' ';
    let mut last_split = 0;
    let bytes = s.as_bytes();

    for (i, &b) in bytes.iter().enumerate() {
        if in_string {
            if b == string_char as u8 && (i == 0 || bytes[i - 1] != b'\\') {
                in_string = false;
            }
            continue;
        }

        match b {
            b'"' | b'\'' => {
                in_string = true;
                string_char = b as char;
            }
            b'(' | b'[' => depth += 1,
            b')' | b']' => depth -= 1,
            b'|' if depth == 0 => {
                parts.push(&s[last_split..i]);
                last_split = i + 1;
            }
            _ => {}
        }
    }
    parts.push(&s[last_split..]);
    parts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plain_text() {
        let tokens = Tokenizer::tokenize("Hello world").unwrap();
        assert_eq!(tokens, vec![Token::Text("Hello world".to_string())]);
    }

    #[test]
    fn test_single_expression() {
        let tokens = Tokenizer::tokenize("Hello {{ name }}!").unwrap();
        assert_eq!(tokens, vec![
            Token::Text("Hello ".to_string()),
            Token::Expression {
                expr: "name".to_string(),
                filters: vec![],
                trim_left: false,
                trim_right: false,
            },
            Token::Text("!".to_string()),
        ]);
    }

    #[test]
    fn test_expression_with_filter() {
        let tokens = Tokenizer::tokenize("{{ name | snake_case }}").unwrap();
        assert_eq!(tokens, vec![
            Token::Expression {
                expr: "name".to_string(),
                filters: vec![Filter { name: "snake_case".to_string() }],
                trim_left: false,
                trim_right: false,
            },
        ]);
    }

    #[test]
    fn test_filter_chain() {
        let tokens = Tokenizer::tokenize("{{ name | snake_case | upper }}").unwrap();
        assert_eq!(tokens, vec![
            Token::Expression {
                expr: "name".to_string(),
                filters: vec![
                    Filter { name: "snake_case".to_string() },
                    Filter { name: "upper".to_string() },
                ],
                trim_left: false,
                trim_right: false,
            },
        ]);
    }

    #[test]
    fn test_dotted_expression() {
        let tokens = Tokenizer::tokenize("{{ entity.name.pascal }}").unwrap();
        assert_eq!(tokens, vec![
            Token::Expression {
                expr: "entity.name.pascal".to_string(),
                filters: vec![],
                trim_left: false,
                trim_right: false,
            },
        ]);
    }

    #[test]
    fn test_logic_block() {
        let tokens = Tokenizer::tokenize("{% for i, x in ipairs(t) do %}").unwrap();
        assert_eq!(tokens, vec![
            Token::Logic {
                code: "for i, x in ipairs(t) do".to_string(),
                trim_left: false,
                trim_right: false,
            },
        ]);
    }

    #[test]
    fn test_comment() {
        let tokens = Tokenizer::tokenize("before{# comment #}after").unwrap();
        assert_eq!(tokens, vec![
            Token::Text("before".to_string()),
            Token::Comment,
            Token::Text("after".to_string()),
        ]);
    }

    #[test]
    fn test_whitespace_trim_markers() {
        let tokens = Tokenizer::tokenize("{{- name -}}").unwrap();
        assert_eq!(tokens, vec![
            Token::Expression {
                expr: "name".to_string(),
                filters: vec![],
                trim_left: true,
                trim_right: true,
            },
        ]);
    }

    #[test]
    fn test_logic_trim_markers() {
        let tokens = Tokenizer::tokenize("{%- end -%}").unwrap();
        assert_eq!(tokens, vec![
            Token::Logic {
                code: "end".to_string(),
                trim_left: true,
                trim_right: true,
            },
        ]);
    }

    #[test]
    fn test_mixed_template() {
        let template = "Hello {{ name }}!\n{% if show then %}\nWelcome\n{% end %}";
        let tokens = Tokenizer::tokenize(template).unwrap();
        assert_eq!(tokens.len(), 6);
        assert!(matches!(tokens[0], Token::Text(_)));
        assert!(matches!(tokens[1], Token::Expression { .. }));
        assert!(matches!(tokens[2], Token::Text(_)));
        assert!(matches!(tokens[3], Token::Logic { .. }));
        assert!(matches!(tokens[4], Token::Text(_)));
        assert!(matches!(tokens[5], Token::Logic { .. }));
    }

    #[test]
    fn test_unterminated_expression() {
        let result = Tokenizer::tokenize("Hello {{ name");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TemplateCompileError::UnterminatedExpression { line: 1 }));
    }

    #[test]
    fn test_unterminated_logic() {
        let result = Tokenizer::tokenize("{% for x in");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TemplateCompileError::UnterminatedLogic { line: 1 }));
    }

    #[test]
    fn test_empty_expression() {
        let result = Tokenizer::tokenize("{{ }}");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TemplateCompileError::EmptyExpression { .. }));
    }

    #[test]
    fn test_lone_brace_is_text() {
        let tokens = Tokenizer::tokenize("{ not a delimiter }").unwrap();
        assert_eq!(tokens, vec![
            Token::Text("{".to_string()),
            Token::Text(" not a delimiter }".to_string()),
        ]);
    }

    #[test]
    fn test_brace_at_end() {
        let tokens = Tokenizer::tokenize("hello {").unwrap();
        assert_eq!(tokens, vec![Token::Text("hello {".to_string())]);
    }
}
