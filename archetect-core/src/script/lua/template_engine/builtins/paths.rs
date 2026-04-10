//! Path manipulation built-in filters and functions.
//!
//! Operate on POSIX-style forward-slash paths because templates are usually
//! generating cross-platform code (manifests, configs, etc.) and `/` is
//! universally accepted on Windows tooling. Authors who need OS-native
//! semantics can fall back to raw Lua.

use mlua::{Lua, Result as LuaResult, Table, Variadic, Value};

pub fn register(lua: &Lua, filters: &Table) -> LuaResult<()> {
    // path_join(a, b, c, ...) — join path segments with `/`, collapsing
    // duplicate separators.
    filters.set(
        "path_join",
        lua.create_function(|_, parts: Variadic<Value>| {
            let mut out = String::new();
            for part in parts {
                let s = match part {
                    Value::String(s) => s.to_string_lossy().to_string(),
                    Value::Nil => continue,
                    other => {
                        return Err(mlua::Error::RuntimeError(format!(
                            "filter `path_join`: expected string segments, got {}",
                            other.type_name()
                        )));
                    }
                };
                let s = s.trim_matches('/');
                if s.is_empty() {
                    continue;
                }
                if !out.is_empty() {
                    out.push('/');
                }
                out.push_str(s);
            }
            Ok(out)
        })?,
    )?;

    // basename(p) — final path component (filename)
    filters.set(
        "basename",
        lua.create_function(|_, p: String| {
            let trimmed = p.trim_end_matches('/');
            Ok(match trimmed.rfind('/') {
                Some(idx) => trimmed[idx + 1..].to_string(),
                None => trimmed.to_string(),
            })
        })?,
    )?;

    // dirname(p) — everything before the final path component
    filters.set(
        "dirname",
        lua.create_function(|_, p: String| {
            let trimmed = p.trim_end_matches('/');
            Ok(match trimmed.rfind('/') {
                Some(idx) => trimmed[..idx].to_string(),
                None => String::new(),
            })
        })?,
    )?;

    // extname(p) — file extension including the leading dot, or empty string
    filters.set(
        "extname",
        lua.create_function(|_, p: String| {
            let base = match p.rfind('/') {
                Some(idx) => &p[idx + 1..],
                None => p.as_str(),
            };
            // Leading dots (e.g. ".gitignore") are not extensions
            let dot = base
                .char_indices()
                .skip(1)
                .filter(|(_, c)| *c == '.')
                .last();
            Ok(match dot {
                Some((idx, _)) => base[idx..].to_string(),
                None => String::new(),
            })
        })?,
    )?;

    // path_normalize(p) — collapse `.` and `..` segments, deduplicate
    // separators. Does NOT touch the filesystem.
    filters.set(
        "path_normalize",
        lua.create_function(|_, p: String| {
            let absolute = p.starts_with('/');
            let mut stack: Vec<&str> = Vec::new();
            for segment in p.split('/') {
                match segment {
                    "" | "." => {}
                    ".." => {
                        if let Some(last) = stack.last() {
                            if *last != ".." {
                                stack.pop();
                                continue;
                            }
                        }
                        if !absolute {
                            stack.push("..");
                        }
                    }
                    other => stack.push(other),
                }
            }
            let mut out = if absolute { "/".to_string() } else { String::new() };
            out.push_str(&stack.join("/"));
            if out.is_empty() {
                out.push('.');
            }
            Ok(out)
        })?,
    )?;

    Ok(())
}
