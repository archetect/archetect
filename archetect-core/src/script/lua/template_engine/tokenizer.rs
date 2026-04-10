use super::error::TemplateCompileError;

#[derive(Debug, Clone, PartialEq)]
pub struct Filter {
    pub name: String,
    /// Raw Lua expressions for additional arguments to the filter, e.g.
    /// `truncate(40, "...")` parses to `args: ["40", "\"...\""]`.
    /// Args are not pre-evaluated — they are emitted verbatim into the
    /// generated Lua, so they resolve through `_ENV → __filters → __ctx`
    /// like any other expression.
    pub args: Vec<String>,
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
    /// `{% include "path/to/file.atl" %}` — special-form logic block. The
    /// path is recorded as parsed (relative to the configured includes
    /// directory) and resolved at compile time by an `IncludeResolver`.
    Include {
        path: String,
        line: usize,
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
                    // `{%` — logic block (or `{% include "..." %}` special form)
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

                            // Special-form: `include "path"`. Recognized at the
                            // tokenizer so a downstream resolver can inline
                            // the file at compile time and so malformed
                            // includes surface as `InvalidInclude` rather than
                            // a confusing Lua parse error.
                            if let Some(rest) = raw.strip_prefix("include") {
                                // Must be followed by whitespace, otherwise
                                // it could be a Lua identifier like `include_xxx`.
                                if rest.starts_with(|c: char| c.is_whitespace()) {
                                    let path = parse_include_path(rest.trim(), start_line)?;
                                    tokens.push(Token::Include {
                                        path,
                                        line: start_line,
                                        trim_left,
                                        trim_right,
                                    });
                                    pos = content_end + 2;
                                    continue;
                                }
                            }

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

/// Parse an `{% include "path" %}` body into the bare path string.
///
/// Accepts double-quoted (`"path"`) and single-quoted (`'path'`) forms.
/// The path content itself is returned verbatim — the resolver layer
/// validates that it stays inside the configured includes directory.
fn parse_include_path(body: &str, line: usize) -> Result<String, TemplateCompileError> {
    let body = body.trim();
    if body.is_empty() {
        return Err(TemplateCompileError::InvalidInclude {
            line,
            detail: "missing path; expected `{% include \"path\" %}`".to_string(),
        });
    }
    let bytes = body.as_bytes();
    let quote = bytes[0];
    if quote != b'"' && quote != b'\'' {
        return Err(TemplateCompileError::InvalidInclude {
            line,
            detail: format!("expected quoted path, got `{}`", body),
        });
    }
    if bytes.len() < 2 || bytes[bytes.len() - 1] != quote {
        return Err(TemplateCompileError::InvalidInclude {
            line,
            detail: "unterminated quoted path".to_string(),
        });
    }
    let path = &body[1..body.len() - 1];
    if path.is_empty() {
        return Err(TemplateCompileError::InvalidInclude {
            line,
            detail: "include path cannot be empty".to_string(),
        });
    }
    Ok(path.to_string())
}

/// Parse an expression string into the base expression and filter chain.
/// Handles `expr | filter1 | filter2(arg1, arg2)`.
fn parse_expression(raw: &str, line: usize) -> Result<(String, Vec<Filter>), TemplateCompileError> {
    // Split on `|` but respect nested parentheses, brackets, and strings
    let parts = split_filters(raw);

    let expr = parts[0].trim().to_string();
    if expr.is_empty() {
        return Err(TemplateCompileError::EmptyExpression { line });
    }

    let mut filters = Vec::new();
    for part in &parts[1..] {
        let segment = part.trim();
        if segment.is_empty() {
            return Err(TemplateCompileError::InvalidFilter {
                line,
                detail: "empty filter name".to_string(),
            });
        }
        filters.push(parse_filter(segment, line)?);
    }

    Ok((expr, filters))
}

/// Parse a single filter segment into name and (optional) argument list.
///
/// Examples:
///   `snake_case`              → Filter { name: "snake_case", args: [] }
///   `truncate(40)`            → Filter { name: "truncate",   args: ["40"] }
///   `truncate(40, "...")`     → Filter { name: "truncate",   args: ["40", "\"...\""] }
///   `replace("a", "b")`       → Filter { name: "replace",    args: ["\"a\"", "\"b\""] }
///   `default(other_var)`      → Filter { name: "default",    args: ["other_var"] }
///
/// Args are *not* pre-evaluated — they're substituted verbatim into the
/// generated Lua, so they resolve at render time through the same `_ENV`
/// chain as any other expression.
fn parse_filter(segment: &str, line: usize) -> Result<Filter, TemplateCompileError> {
    let segment = segment.trim();
    let bytes = segment.as_bytes();

    // Find the first `(` at the top level (no opening parens precede it).
    let paren_pos = bytes.iter().position(|&b| b == b'(');

    let Some(paren_pos) = paren_pos else {
        // Bare filter name, no args.
        if !is_valid_filter_name(segment) {
            return Err(TemplateCompileError::InvalidFilter {
                line,
                detail: format!("invalid filter name `{}`", segment),
            });
        }
        return Ok(Filter {
            name: segment.to_string(),
            args: Vec::new(),
        });
    };

    let name = segment[..paren_pos].trim();
    if !is_valid_filter_name(name) {
        return Err(TemplateCompileError::InvalidFilter {
            line,
            detail: format!("invalid filter name `{}`", name),
        });
    }

    // The matching `)` must be the LAST character. Anything after it is a
    // syntax error in this segment.
    if !segment.ends_with(')') {
        return Err(TemplateCompileError::InvalidFilter {
            line,
            detail: format!(
                "filter `{}` has unbalanced or trailing characters after argument list",
                name
            ),
        });
    }

    // The arg substring is what's between the parens.
    let args_raw = &segment[paren_pos + 1..segment.len() - 1];
    let args = split_filter_args(args_raw);

    // Empty `()` is allowed and means a zero-arg call (functionally identical
    // to a bare filter name, but explicit).
    Ok(Filter {
        name: name.to_string(),
        args,
    })
}

/// True if `s` is a valid filter identifier: letters/digits/underscores,
/// must not start with a digit.
fn is_valid_filter_name(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_alphanumeric() || c == '_')
}

/// Split a filter argument list on top-level commas, respecting nested
/// parens, brackets, and string literals (single + double quoted, with
/// backslash escapes).
fn split_filter_args(s: &str) -> Vec<String> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    let mut parts: Vec<String> = Vec::new();
    let mut depth = 0i32;
    let mut in_string = false;
    let mut string_char = b' ';
    let mut last_split = 0;
    let bytes = s.as_bytes();

    for (i, &b) in bytes.iter().enumerate() {
        if in_string {
            if b == string_char && (i == 0 || bytes[i - 1] != b'\\') {
                in_string = false;
            }
            continue;
        }

        match b {
            b'"' | b'\'' => {
                in_string = true;
                string_char = b;
            }
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => depth -= 1,
            b',' if depth == 0 => {
                parts.push(s[last_split..i].trim().to_string());
                last_split = i + 1;
            }
            _ => {}
        }
    }
    parts.push(s[last_split..].trim().to_string());
    parts
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
                filters: vec![Filter { name: "snake_case".to_string(), args: vec![] }],
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
                    Filter { name: "snake_case".to_string(), args: vec![] },
                    Filter { name: "upper".to_string(), args: vec![] },
                ],
                trim_left: false,
                trim_right: false,
            },
        ]);
    }

    #[test]
    fn test_filter_with_single_arg() {
        let tokens = Tokenizer::tokenize("{{ name | truncate(40) }}").unwrap();
        assert_eq!(tokens, vec![
            Token::Expression {
                expr: "name".to_string(),
                filters: vec![Filter {
                    name: "truncate".to_string(),
                    args: vec!["40".to_string()],
                }],
                trim_left: false,
                trim_right: false,
            },
        ]);
    }

    #[test]
    fn test_filter_with_multiple_args() {
        let tokens = Tokenizer::tokenize(r#"{{ name | replace("a", "b") }}"#).unwrap();
        assert_eq!(tokens, vec![
            Token::Expression {
                expr: "name".to_string(),
                filters: vec![Filter {
                    name: "replace".to_string(),
                    args: vec![r#""a""#.to_string(), r#""b""#.to_string()],
                }],
                trim_left: false,
                trim_right: false,
            },
        ]);
    }

    #[test]
    fn test_filter_with_nested_paren_arg() {
        // Inner f(g(y)) is one argument as far as the outer filter is concerned.
        let tokens = Tokenizer::tokenize("{{ x | f(g(y)) }}").unwrap();
        assert_eq!(tokens, vec![
            Token::Expression {
                expr: "x".to_string(),
                filters: vec![Filter {
                    name: "f".to_string(),
                    args: vec!["g(y)".to_string()],
                }],
                trim_left: false,
                trim_right: false,
            },
        ]);
    }

    #[test]
    fn test_filter_with_string_containing_comma() {
        // Comma inside a string literal must NOT split the args.
        let tokens = Tokenizer::tokenize(r#"{{ items | join(", ") }}"#).unwrap();
        assert_eq!(tokens, vec![
            Token::Expression {
                expr: "items".to_string(),
                filters: vec![Filter {
                    name: "join".to_string(),
                    args: vec![r#"", ""#.to_string()],
                }],
                trim_left: false,
                trim_right: false,
            },
        ]);
    }

    #[test]
    fn test_filter_with_zero_arg_parens() {
        // Explicit `()` is allowed and is equivalent to a bare filter name.
        let tokens = Tokenizer::tokenize("{{ name | upper_case() }}").unwrap();
        assert_eq!(tokens, vec![
            Token::Expression {
                expr: "name".to_string(),
                filters: vec![Filter {
                    name: "upper_case".to_string(),
                    args: vec![],
                }],
                trim_left: false,
                trim_right: false,
            },
        ]);
    }

    #[test]
    fn test_filter_chain_mixed_args() {
        // {{ name | truncate(10) | upper_case }}
        let tokens = Tokenizer::tokenize("{{ name | truncate(10) | upper_case }}").unwrap();
        assert_eq!(tokens, vec![
            Token::Expression {
                expr: "name".to_string(),
                filters: vec![
                    Filter { name: "truncate".to_string(), args: vec!["10".to_string()] },
                    Filter { name: "upper_case".to_string(), args: vec![] },
                ],
                trim_left: false,
                trim_right: false,
            },
        ]);
    }

    #[test]
    fn test_filter_invalid_name_rejected() {
        let result = Tokenizer::tokenize("{{ name | 123bad }}");
        assert!(matches!(
            result,
            Err(TemplateCompileError::InvalidFilter { .. })
        ));
    }

    #[test]
    fn test_filter_trailing_chars_after_args_rejected() {
        let result = Tokenizer::tokenize("{{ name | truncate(40)oops }}");
        assert!(matches!(
            result,
            Err(TemplateCompileError::InvalidFilter { .. })
        ));
    }

    #[test]
    fn test_split_filter_args_empty() {
        assert_eq!(split_filter_args(""), Vec::<String>::new());
        assert_eq!(split_filter_args("   "), Vec::<String>::new());
    }

    #[test]
    fn test_split_filter_args_single() {
        assert_eq!(split_filter_args("40"), vec!["40".to_string()]);
    }

    #[test]
    fn test_split_filter_args_multiple() {
        assert_eq!(
            split_filter_args(r#"40, "..." , true"#),
            vec!["40".to_string(), r#""...""#.to_string(), "true".to_string()],
        );
    }

    #[test]
    fn test_split_filter_args_respects_nested_parens() {
        assert_eq!(split_filter_args("f(a, b), c"), vec!["f(a, b)".to_string(), "c".to_string()]);
    }

    #[test]
    fn test_split_filter_args_respects_string_commas() {
        assert_eq!(
            split_filter_args(r#""a, b", c"#),
            vec![r#""a, b""#.to_string(), "c".to_string()],
        );
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
