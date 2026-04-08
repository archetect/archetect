use super::tokenizer::{Filter, Token};

pub struct Compiler;

impl Compiler {
    /// Compile a token stream into Lua source code that returns a render function.
    ///
    /// The generated function has signature: `function(__ctx, __filters) -> string`
    ///
    /// Bare names in `{{ }}` expressions resolve against `__ctx` via `_ENV.__index`.
    /// Variables introduced in `{% %}` logic blocks (e.g., loop variables) shadow
    /// context keys naturally through Lua's scoping rules.
    pub fn compile(tokens: &[Token]) -> String {
        let mut lua = String::with_capacity(1024);

        // Function preamble — set up output buffer and _ENV for context resolution
        lua.push_str("return function(__ctx, __filters)\n");
        lua.push_str("    local __out = {}\n");
        // nil is dropped silently — emitting the literal "nil" into a generated source
        // file is far worse than an empty interpolation. Strict mode (Phase 6) will
        // offer fail-on-undefined as an opt-in.
        lua.push_str("    local __w = function(s) if s ~= nil then __out[#__out+1] = tostring(s) end end\n");
        lua.push_str("    local _ENV = setmetatable({\n");
        lua.push_str("        __ctx = __ctx,\n");
        lua.push_str("        __filters = __filters,\n");
        lua.push_str("        __out = __out,\n");
        lua.push_str("        __w = __w,\n");
        lua.push_str("        ipairs = ipairs,\n");
        lua.push_str("        pairs = pairs,\n");
        lua.push_str("        tostring = tostring,\n");
        lua.push_str("        tonumber = tonumber,\n");
        lua.push_str("        type = type,\n");
        lua.push_str("        table = table,\n");
        lua.push_str("        string = string,\n");
        lua.push_str("        math = math,\n");
        lua.push_str("        require = require,\n");
        lua.push_str("        print = print,\n");
        lua.push_str("        error = error,\n");
        lua.push_str("        pcall = pcall,\n");
        lua.push_str("        select = select,\n");
        lua.push_str("        unpack = table.unpack or unpack,\n");
        lua.push_str("        next = next,\n");
        lua.push_str("        rawget = rawget,\n");
        lua.push_str("        rawset = rawset,\n");
        lua.push_str("        setmetatable = setmetatable,\n");
        lua.push_str("        getmetatable = getmetatable,\n");
        lua.push_str("    }, {__index = __ctx})\n");
        lua.push_str("\n");

        let token_count = tokens.len();
        for (i, token) in tokens.iter().enumerate() {
            match token {
                Token::Text(text) => {
                    // Apply whitespace trimming from adjacent expression/logic tokens
                    let mut text = text.as_str();

                    // If the previous token had trim_right, strip leading whitespace
                    if i > 0 && has_trim_right(&tokens[i - 1]) {
                        text = trim_leading_whitespace(text);
                    }

                    // If the next token has trim_left, strip trailing whitespace
                    if i + 1 < token_count && has_trim_left(&tokens[i + 1]) {
                        text = trim_trailing_whitespace(text);
                    }

                    if !text.is_empty() {
                        lua.push_str("    __w(\"");
                        lua.push_str(&lua_escape(text));
                        lua.push_str("\")\n");
                    }
                }
                Token::Expression { expr, filters, .. } => {
                    let safe_expr = make_lua_safe(expr);
                    let value_expr = apply_filters(&safe_expr, filters);
                    lua.push_str("    __w(");
                    lua.push_str(&value_expr);
                    lua.push_str(")\n");
                }
                Token::Logic { code, .. } => {
                    lua.push_str("    ");
                    lua.push_str(code);
                    lua.push('\n');
                }
                Token::Comment => {
                    // Comments are stripped — emit nothing
                }
            }
        }

        lua.push_str("\n    return table.concat(__out)\n");
        lua.push_str("end\n");

        lua
    }
}

/// Transform an expression so that hyphenated or otherwise non-Lua-safe identifiers
/// use bracket notation. This is critical because archetect context keys frequently
/// use kebab-case (e.g., `project-name`), which Lua parses as subtraction.
///
/// Examples:
///   `project-name`           → `__ctx["project-name"]`
///   `entity.name.pascal`     → `entity.name.pascal`  (all segments are valid Lua)
///   `entity.field-name`      → `entity["field-name"]`
///   `#entity.local_fields`   → `#entity.local_fields` (Lua length operator)
fn make_lua_safe(expr: &str) -> String {
    let expr = expr.trim();

    // If the expression contains operators, function calls, or brackets, leave it as-is.
    // These are already Lua code that should not be transformed.
    if expr.contains('(') || expr.contains('[') || expr.contains(' ')
        || expr.contains('+') || expr.contains('*') || expr.contains('/')
        || expr.contains('%') || expr.contains('"') || expr.contains('\'')
        || expr.contains(',')
    {
        return expr.to_string();
    }

    // Handle Lua prefix operators (# for length, - for negation, not)
    let (prefix, rest) = if let Some(stripped) = expr.strip_prefix('#') {
        ("#", stripped)
    } else if let Some(stripped) = expr.strip_prefix('-') {
        // Only treat as prefix negation if followed by an identifier, not a digit
        if stripped.starts_with(|c: char| c.is_alphabetic() || c == '_') {
            ("-", stripped)
        } else {
            return expr.to_string();
        }
    } else {
        ("", expr)
    };

    // Split on `.` and handle each segment
    let segments: Vec<&str> = rest.split('.').collect();
    if segments.is_empty() {
        return expr.to_string();
    }

    let mut result = String::new();
    result.push_str(prefix);

    for (i, segment) in segments.iter().enumerate() {
        if i == 0 {
            if is_lua_identifier(segment) {
                result.push_str(segment);
            } else {
                // First segment is not a valid identifier — use __ctx["key"]
                result.push_str("__ctx[\"");
                result.push_str(segment);
                result.push_str("\"]");
            }
        } else if is_lua_identifier(segment) {
            result.push('.');
            result.push_str(segment);
        } else {
            result.push_str("[\"");
            result.push_str(segment);
            result.push_str("\"]");
        }
    }

    result
}

/// Check if a string is a valid Lua identifier (letters, digits, underscores; doesn't start with digit).
fn is_lua_identifier(s: &str) -> bool {
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

/// Wrap an expression with its filter chain.
/// `apply_filters("name", [snake_case, upper])` → `__filters.upper(__filters.snake_case(name))`
fn apply_filters(expr: &str, filters: &[Filter]) -> String {
    if filters.is_empty() {
        return expr.to_string();
    }

    let mut result = expr.to_string();
    for filter in filters {
        result = format!("__filters.{}({})", filter.name, result);
    }
    result
}

/// Escape a string for embedding in a Lua double-quoted string literal.
fn lua_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 16);
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\0' => out.push_str("\\0"),
            c => out.push(c),
        }
    }
    out
}

fn has_trim_right(token: &Token) -> bool {
    match token {
        Token::Expression { trim_right, .. } => *trim_right,
        Token::Logic { trim_right, .. } => *trim_right,
        _ => false,
    }
}

fn has_trim_left(token: &Token) -> bool {
    match token {
        Token::Expression { trim_left, .. } => *trim_left,
        Token::Logic { trim_left, .. } => *trim_left,
        _ => false,
    }
}

/// Trim leading whitespace up to and including the first newline.
fn trim_leading_whitespace(s: &str) -> &str {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b' ' | b'\t' | b'\r' => i += 1,
            b'\n' => return &s[i + 1..],
            _ => break,
        }
    }
    // No newline found — trim all leading whitespace
    &s[i..]
}

/// Trim trailing whitespace up to and including the last newline.
fn trim_trailing_whitespace(s: &str) -> &str {
    let bytes = s.as_bytes();
    let mut i = bytes.len();
    while i > 0 {
        match bytes[i - 1] {
            b' ' | b'\t' | b'\r' => i -= 1,
            b'\n' => return &s[..i - 1],
            _ => break,
        }
    }
    // No newline found — trim all trailing whitespace
    &s[..i]
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::tokenizer::Tokenizer;

    #[test]
    fn test_lua_escape() {
        assert_eq!(lua_escape(r#"He said "hello""#), r#"He said \"hello\""#);
        assert_eq!(lua_escape("line1\nline2"), "line1\\nline2");
        assert_eq!(lua_escape("tab\there"), "tab\\there");
        assert_eq!(lua_escape(r"back\slash"), r"back\\slash");
    }

    #[test]
    fn test_apply_filters_none() {
        assert_eq!(apply_filters("name", &[]), "name");
    }

    #[test]
    fn test_apply_filters_one() {
        let filters = vec![Filter { name: "upper".to_string() }];
        assert_eq!(apply_filters("name", &filters), "__filters.upper(name)");
    }

    #[test]
    fn test_apply_filters_chain() {
        let filters = vec![
            Filter { name: "snake_case".to_string() },
            Filter { name: "upper".to_string() },
        ];
        assert_eq!(
            apply_filters("name", &filters),
            "__filters.upper(__filters.snake_case(name))"
        );
    }

    #[test]
    fn test_compile_text_only() {
        let tokens = Tokenizer::tokenize("Hello world").unwrap();
        let lua = Compiler::compile(&tokens);
        assert!(lua.contains(r#"__w("Hello world")"#));
    }

    #[test]
    fn test_compile_expression() {
        let tokens = Tokenizer::tokenize("Hello {{ name }}!").unwrap();
        let lua = Compiler::compile(&tokens);
        assert!(lua.contains(r#"__w("Hello ")"#));
        assert!(lua.contains("__w(name)"));
        assert!(lua.contains(r#"__w("!")"#));
    }

    #[test]
    fn test_compile_expression_with_filter() {
        let tokens = Tokenizer::tokenize("{{ name | snake_case }}").unwrap();
        let lua = Compiler::compile(&tokens);
        assert!(lua.contains("__w(__filters.snake_case(name))"));
    }

    #[test]
    fn test_compile_logic_block() {
        let tokens = Tokenizer::tokenize("{% for i, x in ipairs(items) do %}\n{{ x }}\n{% end %}").unwrap();
        let lua = Compiler::compile(&tokens);
        assert!(lua.contains("for i, x in ipairs(items) do"));
        assert!(lua.contains("__w(x)"));
        assert!(lua.contains("    end"));
    }

    #[test]
    fn test_compile_comment_stripped() {
        let tokens = Tokenizer::tokenize("before{# comment #}after").unwrap();
        let lua = Compiler::compile(&tokens);
        assert!(lua.contains(r#"__w("before")"#));
        assert!(lua.contains(r#"__w("after")"#));
        assert!(!lua.contains("comment"));
    }

    #[test]
    fn test_compile_dotted_access() {
        let tokens = Tokenizer::tokenize("{{ entity.name.pascal }}").unwrap();
        let lua = Compiler::compile(&tokens);
        assert!(lua.contains("__w(entity.name.pascal)"));
    }

    #[test]
    fn test_make_lua_safe_simple() {
        assert_eq!(make_lua_safe("name"), "name");
        assert_eq!(make_lua_safe("entity.name.pascal"), "entity.name.pascal");
    }

    #[test]
    fn test_make_lua_safe_hyphenated() {
        assert_eq!(make_lua_safe("project-name"), "__ctx[\"project-name\"]");
        assert_eq!(make_lua_safe("org-title"), "__ctx[\"org-title\"]");
    }

    #[test]
    fn test_make_lua_safe_mixed_dotted() {
        assert_eq!(make_lua_safe("entity.field-name"), "entity[\"field-name\"]");
    }

    #[test]
    fn test_make_lua_safe_length_operator() {
        assert_eq!(make_lua_safe("#entity.local_fields"), "#entity.local_fields");
    }

    #[test]
    fn test_make_lua_safe_complex_expression() {
        // Expressions with spaces/operators should be left as-is
        assert_eq!(make_lua_safe("#entity.local_fields + i"), "#entity.local_fields + i");
    }

    #[test]
    fn test_compile_hyphenated_key() {
        let tokens = Tokenizer::tokenize("{{ project-name }}").unwrap();
        let lua = Compiler::compile(&tokens);
        assert!(lua.contains("__w(__ctx[\"project-name\"])"), "Got: {}", lua);
    }

    #[test]
    fn test_compile_full_template() {
        let template = r#"syntax = "proto3";

message {{ entity.name.pascal }} {
{% for i, field in ipairs(entity.local_fields) do %}
    {{ field | proto_type }} {{ field.name.snake }} = {{ i }};
{% end %}
}"#;
        let tokens = Tokenizer::tokenize(template).unwrap();
        let lua = Compiler::compile(&tokens);

        // Should contain the key parts
        assert!(lua.contains(r#"__w("syntax = \"proto3\";\n\nmessage ")"#));
        assert!(lua.contains("__w(entity.name.pascal)"));
        assert!(lua.contains("for i, field in ipairs(entity.local_fields) do"));
        assert!(lua.contains("__w(__filters.proto_type(field))"));
        assert!(lua.contains("__w(field.name.snake)"));
        assert!(lua.contains("__w(i)"));
    }

    #[test]
    fn test_compile_generates_valid_function_structure() {
        let tokens = Tokenizer::tokenize("hello").unwrap();
        let lua = Compiler::compile(&tokens);

        assert!(lua.starts_with("return function(__ctx, __filters)"));
        assert!(lua.contains("local __out = {}"));
        assert!(lua.contains("local __w = function(s) if s ~= nil then __out[#__out+1] = tostring(s) end end"));
        assert!(lua.contains("return table.concat(__out)"));
        assert!(lua.trim_end().ends_with("end"));
    }
}
