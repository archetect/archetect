use std::collections::BTreeMap;

use mlua::{Error as LuaError, Lua, Result as LuaResult, Table, UserData, UserDataMethods, Value};

use archetect_api::{
    BoolPromptInfo, ClientMessage, ContextMap, ContextValue, EditorPromptInfo, IntPromptInfo,
    ListPromptInfo, MultiSelectPromptInfo, ScriptMessage, SelectPromptInfo, TextPromptInfo,
};

use crate::archetype::render_context::RenderContext;
use crate::script::lua::cases::{CaseSpec, CaseSpecEntry, CaseSpecList};
use crate::Archetect;

#[derive(Clone, Debug)]
pub struct Context {
    data: BTreeMap<String, ContextValue>,
    archetect: Archetect,
    render_context: RenderContext,
}

impl Context {
    pub fn new(archetect: Archetect, render_context: RenderContext) -> Self {
        // Pre-load answers from render context (now ContextMap)
        let mut data = BTreeMap::new();
        for (key, value) in render_context.answers() {
            data.insert(key.clone(), value.clone());
        }

        Self {
            data,
            archetect,
            render_context,
        }
    }

    /// Convert context data to a ContextMap.
    pub fn to_context_map(&self) -> ContextMap {
        self.data.clone()
    }

    fn use_default(&self, key: &str) -> bool {
        self.archetect.is_headless()
            || self.render_context.use_defaults_all()
            || self.render_context.use_defaults().contains(key)
    }

    fn send_prompt(&self, msg: ScriptMessage) -> LuaResult<ClientMessage> {
        self.archetect
            .request(msg)
            .map_err(|e| LuaError::RuntimeError(format!("IO error: {}", e)))?;
        self.archetect
            .response()
            .map_err(|e| LuaError::RuntimeError(format!("IO error: {}", e)))
    }

    fn store_string_with_cases(&mut self, key: &str, value: &str, cases: &[CaseSpec]) {
        self.data.insert(key.to_string(), ContextValue::String(value.to_string()));
        for spec in cases {
            match spec {
                CaseSpec::Auto(style) => {
                    let new_key = style.transform_key(key);
                    let new_value = style.transform_value(value);
                    self.data.insert(new_key, ContextValue::String(new_value));
                }
                CaseSpec::Fixed { key: fixed_key, style } => {
                    let new_value = style.transform_value(value);
                    self.data.insert(fixed_key.clone(), ContextValue::String(new_value.clone()));
                    let snake = archetect_inflections::to_snake_case(fixed_key);
                    if snake != *fixed_key {
                        self.data.insert(snake, ContextValue::String(new_value.clone()));
                    }
                    let kebab = archetect_inflections::to_kebab_case(fixed_key);
                    if kebab != *fixed_key {
                        self.data.insert(kebab, ContextValue::String(new_value));
                    }
                }
            }
        }
    }

    /// Convert context data to a Lua table for the Lua-native template engine.
    ///
    /// Only the keys explicitly stored in the Context are written. Cases are an
    /// opt-in concept declared via the `cases =` option on prompts and `ctx:set`
    /// (see `cases` module). The previous implementation silently aliased every
    /// key into snake_case and kebab_case variants, which made templates appear
    /// to "just work" against keys that were never actually declared — a footgun
    /// that masked author errors.
    pub fn to_lua_table(&self, lua: &Lua) -> LuaResult<Table> {
        let table = lua.create_table()?;
        for (key, value) in &self.data {
            let lua_value = context_value_to_lua(lua, value)?;
            table.set(key.as_str(), lua_value)?;
        }
        Ok(table)
    }

}

fn context_value_to_lua(lua: &Lua, value: &ContextValue) -> LuaResult<Value> {
    match value {
        ContextValue::String(s) => Ok(Value::String(lua.create_string(s)?)),
        ContextValue::Integer(i) => Ok(Value::Integer(*i)),
        ContextValue::Float(f) => Ok(Value::Number(*f)),
        ContextValue::Boolean(b) => Ok(Value::Boolean(*b)),
        ContextValue::Array(arr) => {
            let table = lua.create_table()?;
            for (i, item) in arr.iter().enumerate() {
                table.set(i + 1, context_value_to_lua(lua, item)?)?;
            }
            Ok(Value::Table(table))
        }
        ContextValue::Map(map) => {
            let table = lua.create_table()?;
            for (k, v) in map {
                table.set(k.as_str(), context_value_to_lua(lua, v)?)?;
            }
            Ok(Value::Table(table))
        }
        ContextValue::Nil => Ok(Value::Nil),
    }
}

/// Convert a Lua value to a ContextValue.
fn lua_value_to_context_value(value: &Value) -> LuaResult<ContextValue> {
    match value {
        Value::String(s) => Ok(ContextValue::String(s.to_string_lossy().to_string())),
        Value::Integer(i) => Ok(ContextValue::Integer(*i)),
        Value::Number(n) => Ok(ContextValue::Float(*n)),
        Value::Boolean(b) => Ok(ContextValue::Boolean(*b)),
        Value::Table(t) => lua_table_to_context_value(t),
        Value::Nil => Ok(ContextValue::Nil),
        _ => Ok(ContextValue::String(format!("{:?}", value))),
    }
}

/// Convert a Lua table to a ContextValue (map or array).
fn lua_table_to_context_value(table: &Table) -> LuaResult<ContextValue> {
    let len = table.raw_len();
    if len > 0 {
        let mut arr = Vec::new();
        for i in 1..=len {
            let v: Value = table.raw_get(i)?;
            arr.push(lua_value_to_context_value(&v)?);
        }
        Ok(ContextValue::Array(arr))
    } else {
        let mut map = ContextMap::new();
        for pair in table.pairs::<Value, Value>() {
            let (k, v) = pair?;
            let key = match &k {
                Value::String(s) => s.to_string_lossy().to_string(),
                Value::Integer(i) => i.to_string(),
                _ => continue,
            };
            map.insert(key, lua_value_to_context_value(&v)?);
        }
        Ok(ContextValue::Map(map))
    }
}

/// Get the answer lookup key from opts. If `answer_key` is specified in opts,
/// use that; otherwise use the prompt's own key.
fn get_answer_key(opts: &Option<Table>, default_key: &str) -> String {
    if let Some(ref opts) = opts {
        if let Ok(Value::String(s)) = opts.get::<Value>("answer_key") {
            return s.to_string_lossy().to_string();
        }
    }
    default_key.to_string()
}

fn get_opt_string(opts: &Table, key: &str) -> LuaResult<Option<String>> {
    match opts.get::<Value>(key)? {
        Value::String(s) => Ok(Some(s.to_string_lossy().to_string())),
        Value::Nil => Ok(None),
        _ => Ok(None),
    }
}

fn get_opt_i64(opts: &Table, key: &str) -> LuaResult<Option<i64>> {
    match opts.get::<Value>(key)? {
        Value::Integer(i) => Ok(Some(i)),
        Value::Nil => Ok(None),
        _ => Ok(None),
    }
}

fn get_opt_bool(opts: &Table, key: &str) -> LuaResult<Option<bool>> {
    match opts.get::<Value>(key)? {
        Value::Boolean(b) => Ok(Some(b)),
        Value::Nil => Ok(None),
        _ => Ok(None),
    }
}

fn get_opt_string_array(opts: &Table, key: &str) -> LuaResult<Option<Vec<String>>> {
    match opts.get::<Value>(key)? {
        Value::Table(t) => {
            let len = t.raw_len();
            let mut out = Vec::with_capacity(len);
            for i in 1..=len {
                match t.raw_get::<Value>(i)? {
                    Value::String(s) => out.push(s.to_string_lossy().to_string()),
                    Value::Nil => {}
                    _ => return Ok(None),
                }
            }
            Ok(Some(out))
        }
        Value::Nil => Ok(None),
        _ => Ok(None),
    }
}

/// Extract CaseSpec list from an opts table's "cases" field.
fn extract_cases(opts: &Option<Table>) -> Vec<CaseSpec> {
    let opts = match opts {
        Some(opts) => opts,
        None => return vec![],
    };

    let cases_value = match opts.get::<Value>("cases") {
        Ok(v) => v,
        Err(_) => return vec![],
    };

    match cases_value {
        Value::UserData(ud) => {
            if let Ok(list) = ud.borrow::<CaseSpecList>() {
                return list.0.clone();
            }
            if let Ok(entry) = ud.borrow::<CaseSpecEntry>() {
                return vec![entry.0.clone()];
            }
            vec![]
        }
        Value::Table(table) => {
            let mut specs = Vec::new();
            for pair in table.sequence_values::<Value>() {
                if let Ok(Value::UserData(ud)) = pair {
                    if let Ok(list) = ud.borrow::<CaseSpecList>() {
                        specs.extend(list.0.clone());
                    } else if let Ok(entry) = ud.borrow::<CaseSpecEntry>() {
                        specs.push(entry.0.clone());
                    }
                }
            }
            specs
        }
        _ => vec![],
    }
}

fn handle_response_string(response: ClientMessage) -> LuaResult<Option<String>> {
    match response {
        ClientMessage::String(s) => Ok(Some(s)),
        ClientMessage::None => Ok(None),
        ClientMessage::Abort => Err(LuaError::RuntimeError("Prompt aborted".to_string())),
        ClientMessage::Error(e) => Err(LuaError::RuntimeError(format!("Prompt error: {}", e))),
        other => Err(LuaError::RuntimeError(format!(
            "Unexpected response: {:?}",
            other
        ))),
    }
}

fn handle_response_int(response: ClientMessage) -> LuaResult<Option<i64>> {
    match response {
        ClientMessage::Integer(i) => Ok(Some(i)),
        ClientMessage::String(s) => s
            .parse::<i64>()
            .map(Some)
            .map_err(|_| LuaError::RuntimeError(format!("Expected integer, got '{}'", s))),
        ClientMessage::None => Ok(None),
        ClientMessage::Abort => Err(LuaError::RuntimeError("Prompt aborted".to_string())),
        ClientMessage::Error(e) => Err(LuaError::RuntimeError(format!("Prompt error: {}", e))),
        other => Err(LuaError::RuntimeError(format!(
            "Unexpected response: {:?}",
            other
        ))),
    }
}

fn handle_response_bool(response: ClientMessage) -> LuaResult<Option<bool>> {
    match response {
        ClientMessage::Boolean(b) => Ok(Some(b)),
        ClientMessage::None => Ok(None),
        ClientMessage::Abort => Err(LuaError::RuntimeError("Prompt aborted".to_string())),
        ClientMessage::Error(e) => Err(LuaError::RuntimeError(format!("Prompt error: {}", e))),
        other => Err(LuaError::RuntimeError(format!(
            "Unexpected response: {:?}",
            other
        ))),
    }
}

fn handle_response_array(response: ClientMessage) -> LuaResult<Option<Vec<String>>> {
    match response {
        ClientMessage::Array(arr) => Ok(Some(arr)),
        ClientMessage::None => Ok(None),
        ClientMessage::Abort => Err(LuaError::RuntimeError("Prompt aborted".to_string())),
        ClientMessage::Error(e) => Err(LuaError::RuntimeError(format!("Prompt error: {}", e))),
        other => Err(LuaError::RuntimeError(format!(
            "Unexpected response: {:?}",
            other
        ))),
    }
}

impl UserData for Context {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        // ctx:get(key) -> value
        methods.add_method("get", |lua, this, key: String| {
            match this.data.get(&key) {
                Some(value) => context_value_to_lua(lua, value),
                None => Ok(Value::Nil),
            }
        });

        // ctx:has(key) -> bool
        methods.add_method("has", |_, this, key: String| Ok(this.data.contains_key(&key)));

        // ctx:contains(key, value) -> bool
        methods.add_method("contains", |_, this, (key, value): (String, String)| {
            match this.data.get(&key) {
                Some(ContextValue::Array(arr)) => {
                    Ok(arr.iter().any(|v| v.as_str() == Some(value.as_str())))
                }
                _ => Ok(false),
            }
        });

        // ctx:set(key, value, opts?)
        methods.add_method_mut("set", |_, this, (key, value, opts): (String, Value, Option<Table>)| {
            let cases = extract_cases(&opts);
            match &value {
                Value::String(s) => {
                    let s = s.to_string_lossy().to_string();
                    this.store_string_with_cases(&key, &s, &cases);
                }
                Value::Integer(i) => {
                    this.data.insert(key, ContextValue::Integer(*i));
                }
                Value::Boolean(b) => {
                    this.data.insert(key, ContextValue::Boolean(*b));
                }
                Value::Table(table) => {
                    let cv = lua_table_to_context_value(table)?;
                    this.data.insert(key, cv);
                }
                Value::Nil => {
                    this.data.insert(key, ContextValue::Nil);
                }
                _ => {
                    let s = format!("{:?}", value);
                    this.store_string_with_cases(&key, &s, &cases);
                }
            }
            Ok(())
        });

        // ctx:prompt_text(message, key, opts?)
        methods.add_method_mut("prompt_text", |_, this, (message, key, opts): (String, String, Option<Table>)| {
            let mut info = TextPromptInfo::new(&message, Some(&key));
            let cases = extract_cases(&opts);

            if let Some(ref opts) = opts {
                info.default = get_opt_string(opts, "default")?;
                info.help = get_opt_string(opts, "help")?;
                info.placeholder = get_opt_string(opts, "placeholder")?;
                info.min = get_opt_i64(opts, "min")?;
                info.max = get_opt_i64(opts, "max")?;
                if let Some(optional) = get_opt_bool(opts, "optional")? {
                    info.optional = optional;
                }
            }

            let answer_key = get_answer_key(&opts, &key);
            if let Some(ContextValue::String(answer)) = this.data.get(&answer_key).cloned() {
                this.store_string_with_cases(&key, &answer, &cases);
                return Ok(());
            }

            if this.use_default(&key) {
                if let Some(ref default) = info.default {
                    this.store_string_with_cases(&key, default, &cases);
                    return Ok(());
                }
                if info.optional {
                    return Ok(());
                }
                return Err(LuaError::RuntimeError(format!(
                    "Headless mode: no answer or default for '{}'", message
                )));
            }

            let response = this.send_prompt(ScriptMessage::PromptForText(info))?;
            if let Some(value) = handle_response_string(response)? {
                this.store_string_with_cases(&key, &value, &cases);
            }
            Ok(())
        });

        // ctx:prompt_int(message, key, opts?)
        methods.add_method_mut("prompt_int", |_, this, (message, key, opts): (String, String, Option<Table>)| {
            let mut info = IntPromptInfo::new(&message, Some(&key));

            if let Some(ref opts) = opts {
                info.default = get_opt_i64(opts, "default")?;
                info.help = get_opt_string(opts, "help")?;
                info.placeholder = get_opt_string(opts, "placeholder")?;
                info.min = get_opt_i64(opts, "min")?;
                info.max = get_opt_i64(opts, "max")?;
                if let Some(optional) = get_opt_bool(opts, "optional")? {
                    info.optional = optional;
                }
            }

            let answer_key = get_answer_key(&opts, &key);
            if let Some(ContextValue::Integer(v)) = this.data.get(&answer_key).cloned() {
                if answer_key != key {
                    this.data.insert(key, ContextValue::Integer(v));
                }
                return Ok(());
            }

            if this.use_default(&key) {
                if let Some(default) = info.default {
                    this.data.insert(key, ContextValue::Integer(default));
                    return Ok(());
                }
                if info.optional {
                    return Ok(());
                }
                return Err(LuaError::RuntimeError(format!(
                    "Headless mode: no answer or default for '{}'", message
                )));
            }

            let response = this.send_prompt(ScriptMessage::PromptForInt(info))?;
            if let Some(value) = handle_response_int(response)? {
                this.data.insert(key, ContextValue::Integer(value));
            }
            Ok(())
        });

        // ctx:prompt_confirm(message, key, opts?)
        methods.add_method_mut("prompt_confirm", |_, this, (message, key, opts): (String, String, Option<Table>)| {
            let mut info = BoolPromptInfo::new(&message, Some(&key));

            if let Some(ref opts) = opts {
                info.default = get_opt_bool(opts, "default")?;
                info.help = get_opt_string(opts, "help")?;
                info.placeholder = get_opt_string(opts, "placeholder")?;
                if let Some(optional) = get_opt_bool(opts, "optional")? {
                    info.optional = optional;
                }
            }

            let answer_key = get_answer_key(&opts, &key);
            if let Some(ContextValue::Boolean(v)) = this.data.get(&answer_key).cloned() {
                if answer_key != key {
                    this.data.insert(key, ContextValue::Boolean(v));
                }
                return Ok(());
            }

            if this.use_default(&key) {
                if let Some(default) = info.default {
                    this.data.insert(key, ContextValue::Boolean(default));
                    return Ok(());
                }
                if info.optional {
                    return Ok(());
                }
                return Err(LuaError::RuntimeError(format!(
                    "Headless mode: no answer or default for '{}'", message
                )));
            }

            let response = this.send_prompt(ScriptMessage::PromptForBool(info))?;
            if let Some(value) = handle_response_bool(response)? {
                this.data.insert(key, ContextValue::Boolean(value));
            }
            Ok(())
        });

        // ctx:prompt_select(message, key, options, opts?)
        methods.add_method_mut("prompt_select", |_, this, (message, key, options, opts): (String, String, Vec<String>, Option<Table>)| {
            let mut info = SelectPromptInfo::new(&message, Some(&key), options);
            let cases = extract_cases(&opts);

            if let Some(ref opts) = opts {
                info.default = get_opt_string(opts, "default")?;
                info.help = get_opt_string(opts, "help")?;
                info.placeholder = get_opt_string(opts, "placeholder")?;
                if let Some(optional) = get_opt_bool(opts, "optional")? {
                    info.optional = optional;
                }
            }

            let answer_key = get_answer_key(&opts, &key);
            if let Some(ContextValue::String(v)) = this.data.get(&answer_key).cloned() {
                this.store_string_with_cases(&key, &v, &cases);
                return Ok(());
            }

            if this.use_default(&key) {
                if let Some(ref default) = info.default {
                    this.store_string_with_cases(&key, default, &cases);
                    return Ok(());
                }
                if info.optional {
                    return Ok(());
                }
                return Err(LuaError::RuntimeError(format!(
                    "Headless mode: no answer or default for '{}'", message
                )));
            }

            let response = this.send_prompt(ScriptMessage::PromptForSelect(info))?;
            if let Some(value) = handle_response_string(response)? {
                this.store_string_with_cases(&key, &value, &cases);
            }
            Ok(())
        });

        // ctx:prompt_multi_select(message, key, options, opts?)
        methods.add_method_mut("prompt_multi_select", |_, this, (message, key, options, opts): (String, String, Vec<String>, Option<Table>)| {
            let mut info = MultiSelectPromptInfo::new(&message, Some(&key), options);

            if let Some(ref opts) = opts {
                info.help = get_opt_string(opts, "help")?;
                info.placeholder = get_opt_string(opts, "placeholder")?;
                info.min_items = get_opt_i64(opts, "min")?.map(|v| v as usize);
                info.max_items = get_opt_i64(opts, "max")?.map(|v| v as usize);
                info.defaults = get_opt_string_array(opts, "default")?;
                if let Some(optional) = get_opt_bool(opts, "optional")? {
                    info.optional = optional;
                }
            }

            let answer_key = get_answer_key(&opts, &key);
            if let Some(answer) = this.data.get(&answer_key).cloned() {
                match answer {
                    ContextValue::Array(arr) => {
                        if answer_key != key {
                            this.data.insert(key, ContextValue::Array(arr));
                        }
                        return Ok(());
                    }
                    ContextValue::String(s) => {
                        let items: Vec<ContextValue> = s.split(',')
                            .map(|s| ContextValue::String(s.trim().to_string()))
                            .collect();
                        this.data.insert(key, ContextValue::Array(items));
                        return Ok(());
                    }
                    _ => {}
                }
            }

            if this.use_default(&key) {
                if let Some(ref defaults) = info.defaults {
                    let arr: Vec<ContextValue> = defaults.iter()
                        .cloned()
                        .map(ContextValue::String)
                        .collect();
                    this.data.insert(key, ContextValue::Array(arr));
                    return Ok(());
                }
                if info.optional {
                    return Ok(());
                }
                return Err(LuaError::RuntimeError(format!(
                    "Headless mode: no answer or default for '{}'", message
                )));
            }

            let response = this.send_prompt(ScriptMessage::PromptForMultiSelect(info))?;
            if let Some(value) = handle_response_array(response)? {
                let arr: Vec<ContextValue> = value.into_iter().map(ContextValue::String).collect();
                this.data.insert(key, ContextValue::Array(arr));
            }
            Ok(())
        });

        // ctx:prompt_list(message, key, opts?)
        methods.add_method_mut("prompt_list", |_, this, (message, key, opts): (String, String, Option<Table>)| {
            let mut info = ListPromptInfo::new(&message, Some(&key));

            if let Some(ref opts) = opts {
                info.help = get_opt_string(opts, "help")?;
                info.placeholder = get_opt_string(opts, "placeholder")?;
                info.min_items = get_opt_i64(opts, "min")?.map(|v| v as usize);
                info.max_items = get_opt_i64(opts, "max")?.map(|v| v as usize);
                info.defaults = get_opt_string_array(opts, "default")?;
                if let Some(optional) = get_opt_bool(opts, "optional")? {
                    info.optional = optional;
                }
            }

            let answer_key = get_answer_key(&opts, &key);
            if let Some(answer) = this.data.get(&answer_key).cloned() {
                match answer {
                    ContextValue::Array(arr) => {
                        if answer_key != key {
                            this.data.insert(key, ContextValue::Array(arr));
                        }
                        return Ok(());
                    }
                    ContextValue::String(s) => {
                        let items: Vec<ContextValue> = s.split(',')
                            .map(|s| ContextValue::String(s.trim().to_string()))
                            .collect();
                        this.data.insert(key, ContextValue::Array(items));
                        return Ok(());
                    }
                    _ => {}
                }
            }

            if this.use_default(&key) {
                if let Some(ref defaults) = info.defaults {
                    let arr: Vec<ContextValue> = defaults.iter()
                        .cloned()
                        .map(ContextValue::String)
                        .collect();
                    this.data.insert(key, ContextValue::Array(arr));
                    return Ok(());
                }
                if info.optional {
                    return Ok(());
                }
                return Err(LuaError::RuntimeError(format!(
                    "Headless mode: no answer or default for '{}'", message
                )));
            }

            let response = this.send_prompt(ScriptMessage::PromptForList(info))?;
            if let Some(value) = handle_response_array(response)? {
                let arr: Vec<ContextValue> = value.into_iter().map(ContextValue::String).collect();
                this.data.insert(key, ContextValue::Array(arr));
            }
            Ok(())
        });

        // ctx:prompt_editor(message, key, opts?)
        methods.add_method_mut("prompt_editor", |_, this, (message, key, opts): (String, String, Option<Table>)| {
            let mut info = EditorPromptInfo::new(&message, Some(&key));

            if let Some(ref opts) = opts {
                info.default = get_opt_string(opts, "default")?;
                info.help = get_opt_string(opts, "help")?;
                info.placeholder = get_opt_string(opts, "placeholder")?;
            }

            let answer_key = get_answer_key(&opts, &key);
            if let Some(ContextValue::String(v)) = this.data.get(&answer_key).cloned() {
                if answer_key != key {
                    this.data.insert(key, ContextValue::String(v));
                }
                return Ok(());
            }

            if this.use_default(&key) {
                if let Some(ref default) = info.default {
                    this.data.insert(key, ContextValue::String(default.clone()));
                    return Ok(());
                }
                if info.optional {
                    return Ok(());
                }
                return Err(LuaError::RuntimeError(format!(
                    "Headless mode: no answer or default for '{}'", message
                )));
            }

            let response = this.send_prompt(ScriptMessage::PromptForEditor(info))?;
            if let Some(value) = handle_response_string(response)? {
                this.data.insert(key, ContextValue::String(value));
            }
            Ok(())
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use camino::Utf8PathBuf;

    use crate::system::RootedSystemLayout;

    fn make_context() -> Context {
        let layout = RootedSystemLayout::temp().unwrap();
        let archetect = Archetect::builder().with_layout(layout).build().unwrap();
        let render_context = RenderContext::new(Utf8PathBuf::from("/tmp/test"), ContextMap::new());
        Context::new(archetect, render_context)
    }

    #[test]
    fn test_to_lua_table_no_implicit_kebab_alias() {
        // ctx:set("project-name", "foo") used to silently expose `project_name`
        // as well via to_lua_table. After Phase 1.3 the only key written is the
        // one the author actually declared.
        let lua = Lua::new();
        let mut context = make_context();
        context
            .data
            .insert("project-name".to_string(), ContextValue::String("foo".to_string()));

        let table = context.to_lua_table(&lua).unwrap();

        let original: Value = table.get("project-name").unwrap();
        assert!(matches!(original, Value::String(_)), "original key should be present");

        let snake_alias: Value = table.get("project_name").unwrap();
        assert!(
            matches!(snake_alias, Value::Nil),
            "implicit snake_case alias should NOT be present, got {:?}",
            snake_alias
        );
    }

    #[test]
    fn test_to_lua_table_no_implicit_kebab_alias_for_snake_input() {
        // The mirror case: a snake_case key should not silently expose its
        // kebab-case variant either.
        let lua = Lua::new();
        let mut context = make_context();
        context
            .data
            .insert("project_name".to_string(), ContextValue::String("foo".to_string()));

        let table = context.to_lua_table(&lua).unwrap();

        let kebab_alias: Value = table.get("project-name").unwrap();
        assert!(
            matches!(kebab_alias, Value::Nil),
            "implicit kebab-case alias should NOT be present, got {:?}",
            kebab_alias
        );
    }

    #[test]
    fn test_to_lua_table_writes_only_declared_keys() {
        let lua = Lua::new();
        let mut context = make_context();
        context
            .data
            .insert("name".to_string(), ContextValue::String("foo".to_string()));
        context
            .data
            .insert("count".to_string(), ContextValue::Integer(42));

        let table = context.to_lua_table(&lua).unwrap();

        // Walk the table and collect keys.
        let mut keys: Vec<String> = Vec::new();
        for pair in table.pairs::<String, Value>() {
            let (k, _) = pair.unwrap();
            keys.push(k);
        }
        keys.sort();
        assert_eq!(keys, vec!["count".to_string(), "name".to_string()]);
    }
}
