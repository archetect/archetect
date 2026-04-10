use super::error::TemplateCompileError;
use super::include_resolver::IncludeResolver;
use super::tokenizer::{Filter, Token, Tokenizer};

/// Compile-time options that influence the generated Lua source.
///
/// These are sourced from the manifest's `templating:` section and
/// threaded through `TemplateCache` so every template compiled for a
/// given archetype render shares them.
#[derive(Debug, Clone, Copy, Default)]
pub struct CompileOptions {
    /// When true, accessing an undefined context variable raises a Lua
    /// error at render time instead of resolving to nil. Maps to
    /// `templating.undefined: strict` in the manifest.
    pub strict: bool,
    /// Strip the first newline after a `{% ... %}` block tag. Maps to
    /// `templating.trim_blocks` in the manifest.
    pub trim_blocks: bool,
    /// Strip leading whitespace on lines that contain only a block tag.
    /// Maps to `templating.lstrip_blocks` in the manifest.
    pub lstrip_blocks: bool,
}

pub struct Compiler;

impl Compiler {
    /// Compile a token stream into Lua source code that returns a render function.
    ///
    /// The generated function has signature: `function(__ctx, __filters) -> string`
    ///
    /// Bare names in `{{ }}` expressions resolve against `__ctx` via `_ENV.__index`.
    /// Variables introduced in `{% %}` logic blocks (e.g., loop variables) shadow
    /// context keys naturally through Lua's scoping rules.
    ///
    /// Phase 4: when a `Token::Include` is encountered, the resolver reads
    /// the included file and the tokens are spliced inline. The included
    /// body shares `__ctx`, `__filters`, `__out`, and `__w` with the outer
    /// template. Cycle detection is handled by the resolver.
    ///
    /// Phase 6: `opts.strict` installs a metatable on `__ctx` so that any
    /// undefined-variable access raises a render-time error.
    pub fn compile(
        tokens: &[Token],
        resolver: &mut IncludeResolver,
        opts: CompileOptions,
    ) -> Result<String, TemplateCompileError> {
        let mut lua = String::with_capacity(1024);

        // Function preamble — set up output buffer and _ENV for context resolution.
        //
        // Resolution chain for bare names like `{{ now() }}` or `{{ name }}`:
        //
        //   _ENV (stdlib + __out/__w/etc.)
        //     └─ __index → __filters
        //                    └─ __index → __ctx
        //
        // This is the filter/function symmetry from Phase 3 of the ATL evolution
        // plan: every entry in `__filters` is reachable both as `{{ x | foo }}`
        // (compiled to `__filters.foo(x)`) AND as `{{ foo(x) }}` (resolved at
        // render time via the metatable chain). One implementation, two surface
        // forms — authors pick whichever reads better.
        //
        // Filters take precedence over context. An author who calls
        // `ctx:set("now", ...)` will not shadow the `now` builtin.
        lua.push_str("return function(__ctx, __filters)\n");
        lua.push_str("    local __out = {}\n");
        // nil is dropped silently — emitting the literal "nil" into a generated source
        // file is far worse than an empty interpolation. Strict mode (Phase 6) will
        // offer fail-on-undefined as an opt-in.
        lua.push_str("    local __w = function(s) if s ~= nil then __out[#__out+1] = tostring(s) end end\n");
        // Strict mode: install a metatable on __ctx that errors when a key
        // is missing. The lookup chain `_ENV → __filters → __ctx → metatable`
        // means undefined bare names like `{{ name }}` reach the metatable
        // function and raise a Lua RuntimeError that surfaces as
        // `RenderError::LuaTemplateRuntimeError` with a clear message.
        //
        // Note: an EXPLICIT nil (e.g., `ctx:set("x", nil)`) is still
        // resolvable — rawget on __ctx returns nil for both "absent" and
        // "explicitly nil". This matches the spec: undefined-vs-nil is a
        // distinction we don't preserve at this layer because Lua tables
        // can't distinguish them.
        if opts.strict {
            lua.push_str(
                "    setmetatable(__ctx, {__index = function(_, k) error(\"undefined template variable: \" .. tostring(k), 2) end})\n",
            );
        }
        // Chain __filters → __ctx so that bare names in `{{ }}` resolve through
        // both. The setmetatable call is per-render but idempotent.
        lua.push_str("    setmetatable(__filters, {__index = __ctx})\n");
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
        lua.push_str("    }, {__index = __filters})\n");
        lua.push_str("\n");

        compile_body(tokens, resolver, opts, &mut lua)?;

        lua.push_str("\n    return table.concat(__out)\n");
        lua.push_str("end\n");

        Ok(lua)
    }
}

/// Emit body statements for a token stream into `lua`. Recursively splices
/// included templates inline so they share `__ctx`/`__filters`/`__out`/`__w`
/// with the outer template.
fn compile_body(
    tokens: &[Token],
    resolver: &mut IncludeResolver,
    opts: CompileOptions,
    lua: &mut String,
) -> Result<(), TemplateCompileError> {
    let token_count = tokens.len();
    for (i, token) in tokens.iter().enumerate() {
        match token {
            Token::Text(text) => {
                // Apply whitespace trimming from adjacent expression/logic/include tokens
                let owned: Option<String>;
                let mut text_ref: &str = text.as_str();

                // If the previous token had explicit trim_right, OR the
                // previous token is a logic/include AND trim_blocks is on,
                // strip the first newline at the start of this text.
                if i > 0 {
                    let prev = &tokens[i - 1];
                    if has_trim_right(prev)
                        || (opts.trim_blocks && is_block_token(prev))
                    {
                        text_ref = trim_leading_whitespace(text_ref);
                    }
                }

                // If the next token has explicit trim_left, strip trailing
                // whitespace up to and including the last newline.
                if i + 1 < token_count && has_trim_left(&tokens[i + 1]) {
                    text_ref = trim_trailing_whitespace(text_ref);
                }

                // lstrip_blocks: if the next token is a logic/include, strip
                // the spaces/tabs immediately preceding it on its own line.
                // Operates on the line containing the next block tag.
                if opts.lstrip_blocks
                    && i + 1 < token_count
                    && is_block_token(&tokens[i + 1])
                {
                    let stripped = lstrip_block_tail(text_ref);
                    if stripped.len() != text_ref.len() {
                        owned = Some(stripped.to_string());
                        // SAFETY: `owned` lives for the rest of this match arm.
                        text_ref = owned.as_deref().unwrap();
                    } else {
                        owned = None;
                    }
                } else {
                    owned = None;
                }
                let _ = owned;

                if !text_ref.is_empty() {
                    lua.push_str("    __w(\"");
                    lua.push_str(&lua_escape(text_ref));
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
                // Phase 7 sugar: try to rewrite the body into more
                // ergonomic Lua. If no sugar pattern matches, the body
                // passes through verbatim as raw Lua.
                let rewritten = rewrite_sugar(code);
                lua.push_str("    ");
                lua.push_str(rewritten.as_deref().unwrap_or(code));
                lua.push('\n');
            }
            Token::Include { path, line, .. } => {
                // Resolve and read the included file via the resolver
                // (which checks the cycle stack and the includes-dir
                // sandbox), then recursively splice its body into the
                // current output. The resolver tracks the active stack so
                // cycles are caught here rather than at runtime.
                //
                // Resolver errors (IncludeNotFound, IncludeCycle,
                // IncludeReadError) propagate WITHOUT wrapping — they
                // already carry the relevant `path` field. Phase 8.4
                // wrapping only applies to errors that originate INSIDE
                // the partial template's body (tokenize + recursive
                // compile), so the user can tell which partial actually
                // contains the malformed content.
                let (contents, _resolved) = resolver.read(path, *line)?;
                let wrap = |source| TemplateCompileError::IncludeChain {
                    include_path: path.clone(),
                    source: Box::new(source),
                };
                let nested_tokens = Tokenizer::tokenize(&contents).map_err(wrap)?;
                let result = compile_body(&nested_tokens, resolver, opts, lua);
                resolver.pop();
                result.map_err(|e| match e {
                    // Already chained — leave as-is so the chain reads
                    // outermost-first without re-wrapping.
                    TemplateCompileError::IncludeChain { .. } => {
                        TemplateCompileError::IncludeChain {
                            include_path: path.clone(),
                            source: Box::new(e),
                        }
                    }
                    other => TemplateCompileError::IncludeChain {
                        include_path: path.clone(),
                        source: Box::new(other),
                    },
                })?;
            }
            Token::Comment => {
                // Comments are stripped — emit nothing
            }
        }
    }
    Ok(())
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

/// Phase 7: rewrite ergonomic shorthands inside `{% ... %}` logic blocks.
///
/// Patterns recognized:
///
/// | Sugar                              | Lua                                  |
/// |------------------------------------|--------------------------------------|
/// | `for x in items`                   | `for _, x in ipairs(items) do`       |
/// | `for k, v in items`                | `for k, v in pairs(items) do`        |
/// | `for i in range(N)`                | `for i = 0, (N) - 1 do`              |
/// | `for i in range(A, B)`             | `for i = A, (B) - 1 do`              |
/// | `for i in range(A, B, S)`          | `for i = A, (B) - 1, S do`           |
/// | `set NAME = EXPR`                  | `local NAME = EXPR`                  |
/// | `if EXPR`        *(no `then`)*     | `if EXPR then`                       |
/// | `elseif EXPR`    *(no `then`)*     | `elseif EXPR then`                   |
/// | `endif`                            | `end`                                |
/// | `endfor`                           | `end`                                |
///
/// `range()` mirrors Python/Rust semantics — the upper bound is **exclusive**
/// — so `range(10)` iterates 0..=9 and `range(1, 5)` iterates 1..=4.
/// Authors who want Lua-native inclusive iteration can fall back to raw
/// Lua: `{% for i = 1, 10 do %}`.
///
/// The `endif`/`endfor` sugar is for Jinja-flavored compatibility — both
/// keywords mean the same thing as Lua's `end`. Authors can also write
/// `{% end %}` directly if they prefer Lua-native vocabulary.
///
/// The `if`/`elseif` sugar appends an implicit `then` when the body is
/// missing it, so `{% if x %}` works the same as `{% if x then %}`. The
/// detection rule is conservative: only fires when no `then` keyword is
/// already present in the body, so explicit `if x then` and complex
/// expressions like `if x then y else z end` pass through unchanged.
///
/// The detection rule for non-range `for` sugar is also conservative: only
/// apply when the body does NOT already contain `do`. Numeric for loops
/// (`for i = 1, 10 do`) and explicit-iterator forms
/// (`for k, v in ipairs(items) do`) pass through unchanged so authors can
/// always fall back to raw Lua.
///
/// Returns `Some(rewritten)` if a pattern matched, `None` otherwise.
fn rewrite_sugar(body: &str) -> Option<String> {
    let trimmed = body.trim();

    // Jinja-compat block closers — `endif` and `endfor` both map to `end`.
    if trimmed == "endif" || trimmed == "endfor" {
        return Some("end".to_string());
    }

    // `if EXPR` (without `then`) → `if EXPR then`. Conservative: only fire
    // if there's no existing `then` keyword in the body.
    if let Some(rest) = trimmed.strip_prefix("if ") {
        if !contains_keyword(rest, "then") {
            let rest = rest.trim();
            if !rest.is_empty() {
                return Some(format!("if {} then", rest));
            }
        }
    }
    if let Some(rest) = trimmed.strip_prefix("elseif ") {
        if !contains_keyword(rest, "then") {
            let rest = rest.trim();
            if !rest.is_empty() {
                return Some(format!("elseif {} then", rest));
            }
        }
    }

    // `set NAME = EXPR` → `local NAME = EXPR`
    if let Some(rest) = trimmed.strip_prefix("set ") {
        let rest = rest.trim_start();
        // Validate that what follows looks like `IDENT = EXPR`. We require
        // at least an identifier before the `=` so a Lua variable named
        // `setattr` doesn't accidentally match.
        if let Some(eq_pos) = rest.find('=') {
            let name = rest[..eq_pos].trim();
            if !name.is_empty()
                && is_lua_identifier(name)
                // `==` (equality) is not assignment — leave it alone.
                && !rest[eq_pos..].starts_with("==")
            {
                let expr = rest[eq_pos + 1..].trim();
                if !expr.is_empty() {
                    return Some(format!("local {} = {}", name, expr));
                }
            }
        }
    }

    // `for ...` sugar — only apply if no explicit `do` keyword is present.
    if let Some(rest) = trimmed.strip_prefix("for ") {
        if !contains_keyword(rest, "do") {
            // Find the ` in ` separator. The variable list is on the left,
            // the iterable expression is on the right.
            if let Some(in_pos) = find_keyword(rest, "in") {
                let vars = rest[..in_pos].trim();
                let iterable = rest[in_pos + 2..].trim();
                if !vars.is_empty() && !iterable.is_empty() {
                    // `range(...)` — Python/Rust-style numeric for. Only
                    // applicable to single-var loops, since `for k, v in
                    // range(N)` doesn't make sense.
                    if vars.matches(',').count() == 0 {
                        if let Some(rewritten) = try_range_sugar(vars, iterable) {
                            return Some(rewritten);
                        }
                    }

                    // Single variable: assume sequence iteration → ipairs.
                    // Two variables: assume key/value iteration → pairs.
                    // The author always has the explicit form available
                    // if they want different semantics.
                    let comma_count = vars.matches(',').count();
                    return Some(if comma_count == 0 {
                        format!("for _, {} in ipairs({}) do", vars, iterable)
                    } else if comma_count == 1 {
                        format!("for {} in pairs({}) do", vars, iterable)
                    } else {
                        // More than 2 variables — Lua's generic for can
                        // handle this but we don't try to sugar it.
                        return None;
                    });
                }
            }
        }
    }

    None
}

/// Try to rewrite `for IDENT in range(...)` into a numeric Lua for loop.
/// Returns `None` if `iterable` is not a `range(...)` call.
fn try_range_sugar(var: &str, iterable: &str) -> Option<String> {
    let inner = iterable.strip_prefix("range")?;
    let inner = inner.trim_start();
    let inner = inner.strip_prefix('(')?;
    let inner = inner.strip_suffix(')')?;
    let args = split_top_level_commas(inner);
    match args.len() {
        // range(N) → for i = 0, (N) - 1 do
        1 => Some(format!("for {} = 0, ({}) - 1 do", var, args[0].trim())),
        // range(A, B) → for i = A, (B) - 1 do
        2 => Some(format!(
            "for {} = {}, ({}) - 1 do",
            var,
            args[0].trim(),
            args[1].trim(),
        )),
        // range(A, B, S) → for i = A, (B) - 1, S do
        3 => Some(format!(
            "for {} = {}, ({}) - 1, {} do",
            var,
            args[0].trim(),
            args[1].trim(),
            args[2].trim(),
        )),
        _ => None,
    }
}

/// Split `s` on top-level commas, respecting nested parens, brackets, and
/// string literals. Used by `range()` arg parsing.
fn split_top_level_commas(s: &str) -> Vec<&str> {
    let mut parts = Vec::new();
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
                parts.push(&s[last_split..i]);
                last_split = i + 1;
            }
            _ => {}
        }
    }
    if !s.is_empty() {
        parts.push(&s[last_split..]);
    }
    parts
}

/// True if `text` contains `keyword` as a standalone word (delimited by
/// whitespace, parens, or string boundaries — not embedded in another
/// identifier). Used by sugar detection to check for `do` and `in` keywords
/// without false-matching `door` or `into`.
fn contains_keyword(text: &str, keyword: &str) -> bool {
    find_keyword(text, keyword).is_some()
}

/// Locate `keyword` as a standalone word in `text`. Returns the byte
/// offset of the first match, or `None`. The match must be bounded by
/// non-identifier characters on both sides (or be at the start/end of
/// `text`).
fn find_keyword(text: &str, keyword: &str) -> Option<usize> {
    let bytes = text.as_bytes();
    let kw = keyword.as_bytes();
    if bytes.len() < kw.len() {
        return None;
    }
    let mut i = 0;
    while i + kw.len() <= bytes.len() {
        if &bytes[i..i + kw.len()] == kw {
            let before_ok = i == 0 || !is_ident_byte(bytes[i - 1]);
            let after_ok =
                i + kw.len() == bytes.len() || !is_ident_byte(bytes[i + kw.len()]);
            if before_ok && after_ok {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}

fn is_ident_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Wrap an expression with its filter chain.
///
/// Without args: `[snake_case, upper]` → `__filters.upper(__filters.snake_case(name))`
///
/// With args: `[truncate(40), upper]` → `__filters.upper(__filters.truncate(name, 40))`
///
/// Args are emitted verbatim — they're raw Lua expressions that will be
/// evaluated at render time inside `_ENV`, so things like
/// `{{ x | default(other_var) }}` resolve `other_var` through the same
/// context-lookup chain as any other identifier.
fn apply_filters(expr: &str, filters: &[Filter]) -> String {
    if filters.is_empty() {
        return expr.to_string();
    }

    let mut result = expr.to_string();
    for filter in filters {
        if filter.args.is_empty() {
            result = format!("__filters.{}({})", filter.name, result);
        } else {
            result = format!(
                "__filters.{}({}, {})",
                filter.name,
                result,
                filter.args.join(", ")
            );
        }
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
        Token::Include { trim_right, .. } => *trim_right,
        _ => false,
    }
}

fn has_trim_left(token: &Token) -> bool {
    match token {
        Token::Expression { trim_left, .. } => *trim_left,
        Token::Logic { trim_left, .. } => *trim_left,
        Token::Include { trim_left, .. } => *trim_left,
        _ => false,
    }
}

/// True if the token is a `{% ... %}` block tag — Logic or Include.
/// `trim_blocks` and `lstrip_blocks` only fire around block tags, not
/// around `{{ ... }}` expressions.
fn is_block_token(token: &Token) -> bool {
    matches!(token, Token::Logic { .. } | Token::Include { .. })
}

/// `lstrip_blocks` companion: if the trailing portion of `text` (the part
/// after the last newline, or the whole string if no newline) consists
/// entirely of horizontal whitespace, strip it. This removes the
/// indentation in front of a `{% ... %}` block tag that occupies its own
/// line, without touching content lines.
fn lstrip_block_tail(text: &str) -> &str {
    let bytes = text.as_bytes();
    let last_nl = bytes.iter().rposition(|&b| b == b'\n');
    let tail_start = last_nl.map(|p| p + 1).unwrap_or(0);
    let tail = &text[tail_start..];
    if !tail.is_empty() && tail.bytes().all(|b| b == b' ' || b == b'\t') {
        &text[..tail_start]
    } else {
        text
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

    /// Test helper — runs the compiler with a disabled include resolver,
    /// default options, and unwraps. Tests in this module are only
    /// concerned with non-include constructs; include compilation is
    /// exercised in mod.rs.
    fn compile(tokens: &[Token]) -> String {
        let mut resolver = IncludeResolver::disabled();
        Compiler::compile(tokens, &mut resolver, CompileOptions::default()).unwrap()
    }

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
        let filters = vec![Filter { name: "upper".to_string(), args: vec![] }];
        assert_eq!(apply_filters("name", &filters), "__filters.upper(name)");
    }

    #[test]
    fn test_apply_filters_chain() {
        let filters = vec![
            Filter { name: "snake_case".to_string(), args: vec![] },
            Filter { name: "upper".to_string(), args: vec![] },
        ];
        assert_eq!(
            apply_filters("name", &filters),
            "__filters.upper(__filters.snake_case(name))"
        );
    }

    #[test]
    fn test_apply_filters_with_args() {
        let filters = vec![Filter {
            name: "truncate".to_string(),
            args: vec!["40".to_string(), "\"...\"".to_string()],
        }];
        assert_eq!(
            apply_filters("name", &filters),
            "__filters.truncate(name, 40, \"...\")"
        );
    }

    #[test]
    fn test_apply_filters_chain_with_args() {
        let filters = vec![
            Filter { name: "truncate".to_string(), args: vec!["10".to_string()] },
            Filter { name: "upper".to_string(), args: vec![] },
        ];
        assert_eq!(
            apply_filters("name", &filters),
            "__filters.upper(__filters.truncate(name, 10))"
        );
    }

    #[test]
    fn test_compile_text_only() {
        let tokens = Tokenizer::tokenize("Hello world").unwrap();
        let lua = compile(&tokens);
        assert!(lua.contains(r#"__w("Hello world")"#));
    }

    #[test]
    fn test_compile_expression() {
        let tokens = Tokenizer::tokenize("Hello {{ name }}!").unwrap();
        let lua = compile(&tokens);
        assert!(lua.contains(r#"__w("Hello ")"#));
        assert!(lua.contains("__w(name)"));
        assert!(lua.contains(r#"__w("!")"#));
    }

    #[test]
    fn test_compile_expression_with_filter() {
        let tokens = Tokenizer::tokenize("{{ name | snake_case }}").unwrap();
        let lua = compile(&tokens);
        assert!(lua.contains("__w(__filters.snake_case(name))"));
    }

    #[test]
    fn test_compile_logic_block() {
        let tokens = Tokenizer::tokenize("{% for i, x in ipairs(items) do %}\n{{ x }}\n{% end %}").unwrap();
        let lua = compile(&tokens);
        assert!(lua.contains("for i, x in ipairs(items) do"));
        assert!(lua.contains("__w(x)"));
        assert!(lua.contains("    end"));
    }

    #[test]
    fn test_compile_comment_stripped() {
        let tokens = Tokenizer::tokenize("before{# comment #}after").unwrap();
        let lua = compile(&tokens);
        assert!(lua.contains(r#"__w("before")"#));
        assert!(lua.contains(r#"__w("after")"#));
        assert!(!lua.contains("comment"));
    }

    #[test]
    fn test_compile_dotted_access() {
        let tokens = Tokenizer::tokenize("{{ entity.name.pascal }}").unwrap();
        let lua = compile(&tokens);
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
        let lua = compile(&tokens);
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
        let lua = compile(&tokens);

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
        let lua = compile(&tokens);

        assert!(lua.starts_with("return function(__ctx, __filters)"));
        assert!(lua.contains("local __out = {}"));
        assert!(lua.contains("local __w = function(s) if s ~= nil then __out[#__out+1] = tostring(s) end end"));
        assert!(lua.contains("return table.concat(__out)"));
        assert!(lua.trim_end().ends_with("end"));
    }

    // ---------- Phase 7: sugar rewrites (unit tests on rewrite_sugar) ----------

    #[test]
    fn test_sugar_for_single_var_to_ipairs() {
        assert_eq!(
            rewrite_sugar("for item in items").unwrap(),
            "for _, item in ipairs(items) do"
        );
    }

    #[test]
    fn test_sugar_for_two_var_to_pairs() {
        assert_eq!(
            rewrite_sugar("for k, v in items").unwrap(),
            "for k, v in pairs(items) do"
        );
    }

    #[test]
    fn test_sugar_for_explicit_do_passes_through() {
        // Already-explicit form should NOT be rewritten.
        assert_eq!(rewrite_sugar("for i, x in ipairs(items) do"), None);
    }

    #[test]
    fn test_sugar_for_numeric_passes_through() {
        // Numeric for is already valid Lua, no sugar needed.
        assert_eq!(rewrite_sugar("for i = 1, 10 do"), None);
    }

    #[test]
    fn test_sugar_set_simple() {
        assert_eq!(
            rewrite_sugar("set name = \"value\"").unwrap(),
            "local name = \"value\""
        );
    }

    #[test]
    fn test_sugar_set_does_not_match_equality() {
        assert_eq!(rewrite_sugar("set name == \"value\""), None);
    }

    #[test]
    fn test_sugar_set_complex_expr() {
        assert_eq!(
            rewrite_sugar("set total = a + b * 2").unwrap(),
            "local total = a + b * 2"
        );
    }

    #[test]
    fn test_sugar_range_one_arg() {
        assert_eq!(
            rewrite_sugar("for i in range(10)").unwrap(),
            "for i = 0, (10) - 1 do"
        );
    }

    #[test]
    fn test_sugar_range_two_args() {
        assert_eq!(
            rewrite_sugar("for i in range(1, 5)").unwrap(),
            "for i = 1, (5) - 1 do"
        );
    }

    #[test]
    fn test_sugar_range_three_args() {
        assert_eq!(
            rewrite_sugar("for i in range(0, 10, 2)").unwrap(),
            "for i = 0, (10) - 1, 2 do"
        );
    }

    #[test]
    fn test_sugar_range_with_identifier_arg() {
        assert_eq!(
            rewrite_sugar("for i in range(count)").unwrap(),
            "for i = 0, (count) - 1 do"
        );
    }

    #[test]
    fn test_sugar_unrecognized_passes_through() {
        // Bare `end` and `else` are valid Lua already — no rewrite.
        assert_eq!(rewrite_sugar("end"), None);
        assert_eq!(rewrite_sugar("else"), None);
    }

    // ---------- Jinja compatibility sugar ----------

    #[test]
    fn test_sugar_endif_to_end() {
        assert_eq!(rewrite_sugar("endif").unwrap(), "end");
        assert_eq!(rewrite_sugar("  endif  ").unwrap(), "end");
    }

    #[test]
    fn test_sugar_endfor_to_end() {
        assert_eq!(rewrite_sugar("endfor").unwrap(), "end");
    }

    #[test]
    fn test_sugar_if_appends_then() {
        assert_eq!(rewrite_sugar("if x").unwrap(), "if x then");
        assert_eq!(
            rewrite_sugar("if x and y or z").unwrap(),
            "if x and y or z then"
        );
    }

    #[test]
    fn test_sugar_if_explicit_then_passes_through() {
        // Already-explicit `if x then` is valid Lua — no rewrite.
        assert_eq!(rewrite_sugar("if x > 0 then"), None);
        assert_eq!(rewrite_sugar("if x then y else z end"), None);
    }

    #[test]
    fn test_sugar_elseif_appends_then() {
        assert_eq!(rewrite_sugar("elseif x").unwrap(), "elseif x then");
    }

    #[test]
    fn test_sugar_elseif_explicit_then_passes_through() {
        assert_eq!(rewrite_sugar("elseif x then"), None);
    }

    #[test]
    fn test_keyword_detection_word_boundary() {
        // `do` should not match inside `door`
        assert!(!contains_keyword("door open", "do"));
        // `in` should not match inside `into`
        assert!(!contains_keyword("into", "in"));
        // But standalone `do` and `in` should match
        assert!(contains_keyword("for i in items", "in"));
        assert!(contains_keyword("for i = 1, 10 do", "do"));
    }

    #[test]
    fn test_lstrip_block_tail_strips_indent_only_lines() {
        assert_eq!(lstrip_block_tail("hello\n    "), "hello\n");
        assert_eq!(lstrip_block_tail("hello\n\t"), "hello\n");
        // No leading whitespace on the trailing line — leave alone
        assert_eq!(lstrip_block_tail("hello\nworld"), "hello\nworld");
        // Trailing whitespace-only with no preceding newline → strip whole thing
        assert_eq!(lstrip_block_tail("    "), "");
    }
}
