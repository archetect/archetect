//! Collection (array) built-in filters and functions.

use mlua::{Error as LuaError, Lua, Result as LuaResult, Table, Value};

pub fn register(lua: &Lua, filters: &Table) -> LuaResult<()> {
    // join(arr, sep) — concatenate array elements with separator
    filters.set(
        "join",
        lua.create_function(|_, (arr, sep): (Table, String)| {
            let len = arr.raw_len();
            let mut out = String::new();
            for i in 1..=len {
                if i > 1 {
                    out.push_str(&sep);
                }
                let v: Value = arr.raw_get(i)?;
                match v {
                    Value::String(s) => out.push_str(&s.to_string_lossy()),
                    Value::Integer(i) => out.push_str(&i.to_string()),
                    Value::Number(n) => out.push_str(&n.to_string()),
                    Value::Boolean(b) => out.push_str(&b.to_string()),
                    Value::Nil => {}
                    other => {
                        return Err(LuaError::RuntimeError(format!(
                            "filter `join`: array contains non-scalar value of type {}",
                            other.type_name()
                        )));
                    }
                }
            }
            Ok(out)
        })?,
    )?;

    // first(arr) — first element, or nil if empty
    filters.set(
        "first",
        lua.create_function(|_, arr: Table| {
            let v: Value = arr.raw_get(1)?;
            Ok(v)
        })?,
    )?;

    // last(arr) — last element, or nil if empty
    filters.set(
        "last",
        lua.create_function(|_, arr: Table| {
            let len = arr.raw_len();
            if len == 0 {
                return Ok(Value::Nil);
            }
            let v: Value = arr.raw_get(len)?;
            Ok(v)
        })?,
    )?;

    // sort(arr) — return a sorted COPY of the input. Original is untouched.
    filters.set(
        "sort",
        lua.create_function(|lua, arr: Table| {
            let len = arr.raw_len();
            let mut items: Vec<Value> = Vec::with_capacity(len);
            for i in 1..=len {
                items.push(arr.raw_get(i)?);
            }
            // Sort by string representation. This is a simple, predictable
            // ordering that works for both numeric and string arrays. For
            // mixed-type arrays, fall back to type-name ordering.
            items.sort_by_key(sort_key);
            let out = lua.create_table()?;
            for (i, item) in items.into_iter().enumerate() {
                out.set(i + 1, item)?;
            }
            Ok(out)
        })?,
    )?;

    // reverse(arr) — return a reversed copy
    filters.set(
        "reverse",
        lua.create_function(|lua, arr: Table| {
            let len = arr.raw_len();
            let out = lua.create_table()?;
            for i in 1..=len {
                let v: Value = arr.raw_get(i)?;
                out.set(len - i + 1, v)?;
            }
            Ok(out)
        })?,
    )?;

    // contains(haystack, needle) — true if needle is found in haystack.
    //
    // Works on:
    //   - tables (array form): membership test, scalar equality
    //   - strings: substring search
    //   - nil: always false (so missing context vars don't blow up)
    //
    // Designed for the Jinja-flavored idiom `{% if "TOC" in features %}`,
    // which converts to `{% if contains(features, "TOC") then %}`.
    filters.set(
        "contains",
        lua.create_function(|_, (haystack, needle): (Value, Value)| {
            match (&haystack, &needle) {
                (Value::Nil, _) => Ok(false),
                (Value::Table(t), n) => {
                    let len = t.raw_len();
                    let needle_key = sort_key(n);
                    for i in 1..=len {
                        let v: Value = t.raw_get(i)?;
                        if sort_key(&v) == needle_key {
                            return Ok(true);
                        }
                    }
                    Ok(false)
                }
                (Value::String(s), Value::String(needle_str)) => {
                    Ok(s.to_string_lossy().contains(&*needle_str.to_string_lossy()))
                }
                (Value::String(s), Value::Integer(i)) => {
                    Ok(s.to_string_lossy().contains(&i.to_string()))
                }
                _ => Err(LuaError::RuntimeError(format!(
                    "filter `contains`: expected table or string haystack, got {}",
                    haystack.type_name()
                ))),
            }
        })?,
    )?;

    // unique(arr) — return a deduplicated copy preserving first-occurrence order
    filters.set(
        "unique",
        lua.create_function(|lua, arr: Table| {
            let len = arr.raw_len();
            let out = lua.create_table()?;
            let mut seen: Vec<String> = Vec::with_capacity(len);
            let mut next = 1;
            for i in 1..=len {
                let v: Value = arr.raw_get(i)?;
                let key = sort_key(&v);
                if !seen.contains(&key) {
                    seen.push(key);
                    out.set(next, v)?;
                    next += 1;
                }
            }
            Ok(out)
        })?,
    )?;

    Ok(())
}

/// Project a Lua value to a string for ordering / equality. This is
/// deliberately simple — sort/unique are best-effort over scalar arrays.
fn sort_key(value: &Value) -> String {
    match value {
        Value::String(s) => format!("s:{}", s.to_string_lossy()),
        Value::Integer(i) => format!("i:{:020}", i),
        Value::Number(n) => format!("n:{}", n),
        Value::Boolean(b) => format!("b:{}", b),
        Value::Nil => "z:nil".to_string(),
        other => format!("?:{}", other.type_name()),
    }
}
