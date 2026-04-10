//! String built-in filters and functions.
//!
//! Every entry registered here is reachable in templates as both:
//!   `{{ s | foo(...) }}`   — pipe form
//!   `{{ foo(s, ...) }}`    — function form
//!
//! Per the filter/function symmetry principle (Phase 3.0), there is exactly
//! one Rust implementation per name and the compiler routes both surface
//! forms to the same `__filters.<name>` lookup.

use mlua::{Error as LuaError, Lua, Result as LuaResult, Table, Value, Variadic};

pub fn register(lua: &Lua, filters: &Table) -> LuaResult<()> {
    // default(value, fallback) — returns fallback if value is nil or empty string
    filters.set(
        "default",
        lua.create_function(|_, (value, fallback): (Value, Value)| {
            let use_fallback = match &value {
                Value::Nil => true,
                Value::String(s) => s.to_string_lossy().is_empty(),
                _ => false,
            };
            Ok(if use_fallback { fallback } else { value })
        })?,
    )?;

    // truncate(s, n, suffix?) — truncate to n chars; appends suffix (default "…")
    // when truncation occurred. Counts Unicode code points (chars), not bytes.
    filters.set(
        "truncate",
        lua.create_function(
            |_, (s, n, suffix): (String, i64, Option<String>)| {
                if n < 0 {
                    return Err(LuaError::RuntimeError(format!(
                        "filter `truncate`: length must be non-negative, got {}",
                        n
                    )));
                }
                let n = n as usize;
                let chars: Vec<char> = s.chars().collect();
                if chars.len() <= n {
                    return Ok(s);
                }
                let mut out: String = chars.into_iter().take(n).collect();
                out.push_str(&suffix.unwrap_or_else(|| "…".to_string()));
                Ok(out)
            },
        )?,
    )?;

    // replace(s, from, to) — string replace, all occurrences
    filters.set(
        "replace",
        lua.create_function(|_, (s, from, to): (String, String, String)| {
            Ok(s.replace(&from, &to))
        })?,
    )?;

    // trim(s) — strip leading and trailing whitespace
    filters.set(
        "trim",
        lua.create_function(|_, s: String| Ok(s.trim().to_string()))?,
    )?;

    filters.set(
        "trim_start",
        lua.create_function(|_, s: String| Ok(s.trim_start().to_string()))?,
    )?;

    filters.set(
        "trim_end",
        lua.create_function(|_, s: String| Ok(s.trim_end().to_string()))?,
    )?;

    // indent(s, n) — prefix every line with n spaces
    filters.set(
        "indent",
        lua.create_function(|_, (s, n): (String, i64)| {
            if n < 0 {
                return Err(LuaError::RuntimeError(format!(
                    "filter `indent`: width must be non-negative, got {}",
                    n
                )));
            }
            let prefix = " ".repeat(n as usize);
            let mut out = String::with_capacity(s.len() + prefix.len() * (s.lines().count() + 1));
            let mut first = true;
            for line in s.split_inclusive('\n') {
                if first {
                    first = false;
                }
                // Don't indent empty trailing lines
                if line == "\n" {
                    out.push('\n');
                } else {
                    out.push_str(&prefix);
                    out.push_str(line);
                }
            }
            // Handle empty input
            if out.is_empty() {
                out.push_str(&prefix);
            }
            Ok(out)
        })?,
    )?;

    // string_repeat(s, n) — repeat a string n times.
    //
    // Note: deliberately not named `repeat` because `repeat` is a Lua reserved
    // word and would break the function-form `{{ repeat(s, n) }}` (the pipe
    // form `{{ s | repeat(n) }}` works because field access is allowed for
    // reserved words). Using `string_repeat` lets both surface forms work.
    filters.set(
        "string_repeat",
        lua.create_function(|_, (s, n): (String, i64)| {
            if n < 0 {
                return Err(LuaError::RuntimeError(format!(
                    "filter `string_repeat`: count must be non-negative, got {}",
                    n
                )));
            }
            Ok(s.repeat(n as usize))
        })?,
    )?;

    // split(s, sep) — split into an array of substrings
    filters.set(
        "split",
        lua.create_function(|lua, (s, sep): (String, String)| {
            let table = lua.create_table()?;
            for (i, part) in s.split(&sep).enumerate() {
                table.set(i + 1, part)?;
            }
            Ok(table)
        })?,
    )?;

    // length(v) — Unicode-character count for strings, element count for arrays.
    //
    // Note: Lua's `#` operator works on both, but using a named filter lets
    // authors write `{{ items | length }}` for symmetry with other filters.
    filters.set(
        "length",
        lua.create_function(|_, value: Value| match value {
            Value::String(s) => Ok(s.to_string_lossy().chars().count() as i64),
            Value::Table(t) => Ok(t.raw_len() as i64),
            Value::Nil => Ok(0),
            other => Err(LuaError::RuntimeError(format!(
                "filter `length`: expected string or array, got {}",
                other.type_name()
            ))),
        })?,
    )?;

    // concat(a, b, c, ...) — join all string args together. Useful for
    // composing prefixes and suffixes inline without resorting to Lua's `..`.
    filters.set(
        "concat",
        lua.create_function(|_, parts: Variadic<Value>| {
            let mut out = String::new();
            for part in parts {
                match part {
                    Value::String(s) => out.push_str(&s.to_string_lossy()),
                    Value::Integer(i) => out.push_str(&i.to_string()),
                    Value::Number(n) => out.push_str(&n.to_string()),
                    Value::Boolean(b) => out.push_str(&b.to_string()),
                    Value::Nil => {}
                    other => {
                        return Err(LuaError::RuntimeError(format!(
                            "filter `concat`: expected scalar values, got {}",
                            other.type_name()
                        )));
                    }
                }
            }
            Ok(out)
        })?,
    )?;

    Ok(())
}
