use std::collections::BTreeMap;

use mlua::{Error as LuaError, Lua, Result as LuaResult, Table, UserData, UserDataMethods, Value};

use archetect_api::{
    BoolPromptInfo, ClientMessage, EditorPromptInfo, IntPromptInfo, ListPromptInfo,
    MultiSelectPromptInfo, ScriptMessage, SelectPromptInfo, TextPromptInfo,
};

use crate::archetype::render_context::RenderContext;
use crate::script::lua::cases::{expand_cases, CaseSpec, CaseSpecEntry, CaseSpecList};
use crate::Archetect;

#[derive(Clone, Debug)]
pub struct Context {
    data: BTreeMap<String, ContextValue>,
    archetect: Archetect,
    render_context: RenderContext,
}

#[derive(Clone, Debug)]
pub enum ContextValue {
    String(String),
    Integer(i64),
    Boolean(bool),
    Array(Vec<String>),
    Nil,
}

impl Context {
    pub fn new(archetect: Archetect, render_context: RenderContext) -> Self {
        // Pre-load answers from render context
        let mut data = BTreeMap::new();
        for (key, value) in render_context.answers() {
            let key_str = key.to_string();
            if let Some(s) = value.clone().try_cast::<String>() {
                data.insert(key_str, ContextValue::String(s));
            } else if let Some(i) = value.clone().try_cast::<i64>() {
                data.insert(key_str, ContextValue::Integer(i));
            } else if let Some(b) = value.clone().try_cast::<bool>() {
                data.insert(key_str, ContextValue::Boolean(b));
            }
        }

        Self {
            data,
            archetect,
            render_context,
        }
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
        for (case_key, case_value) in expand_cases(key, value, cases) {
            self.data.insert(case_key, ContextValue::String(case_value));
        }
    }

    pub fn to_rhai_map(&self) -> rhai::Map {
        let mut map = rhai::Map::new();
        for (key, value) in &self.data {
            let dynamic = match value {
                ContextValue::String(s) => rhai::Dynamic::from(s.clone()),
                ContextValue::Integer(i) => rhai::Dynamic::from(*i),
                ContextValue::Boolean(b) => rhai::Dynamic::from(*b),
                ContextValue::Array(arr) => {
                    let array: rhai::Array =
                        arr.iter().map(|s| rhai::Dynamic::from(s.clone())).collect();
                    rhai::Dynamic::from(array)
                }
                ContextValue::Nil => rhai::Dynamic::UNIT,
            };
            // Store with original key
            map.insert(key.clone().into(), dynamic.clone());
            // Also store with snake_case key for template compatibility
            let snake = archetect_inflections::to_snake_case(key);
            if snake != *key {
                map.insert(snake.into(), dynamic);
            }
        }
        map
    }
}

fn context_value_to_lua(lua: &Lua, value: &ContextValue) -> LuaResult<Value> {
    match value {
        ContextValue::String(s) => Ok(Value::String(lua.create_string(s)?)),
        ContextValue::Integer(i) => Ok(Value::Integer(*i)),
        ContextValue::Boolean(b) => Ok(Value::Boolean(*b)),
        ContextValue::Array(arr) => {
            let table = lua.create_table()?;
            for (i, s) in arr.iter().enumerate() {
                table.set(i + 1, lua.create_string(s)?)?;
            }
            Ok(Value::Table(table))
        }
        ContextValue::Nil => Ok(Value::Nil),
    }
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

/// Extract CaseSpec list from an opts table's "cases" field.
/// The field can be:
/// - A CaseSpecList (from Cases.programming(), Cases.all(), Cases.set())
/// - A table containing CaseSpecList and/or CaseSpecEntry items
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
            // Direct CaseSpecList (e.g., cases = Cases.programming())
            if let Ok(list) = ud.borrow::<CaseSpecList>() {
                return list.0.clone();
            }
            // Single CaseSpecEntry (e.g., cases = Cases.fixed("key", "title"))
            if let Ok(entry) = ud.borrow::<CaseSpecEntry>() {
                return vec![entry.0.clone()];
            }
            vec![]
        }
        Value::Table(table) => {
            // Mixed list: { Cases.programming(), Cases.fixed("key", "title") }
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

        // ctx:text(message, key, opts?)
        methods.add_method_mut("text", |_, this, (message, key, opts): (String, String, Option<Table>)| {
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

            // Check for pre-supplied answer
            if let Some(ContextValue::String(answer)) = this.data.get(&key).cloned() {
                this.store_string_with_cases(&key, &answer, &cases);
                return Ok(());
            }

            // Check headless/defaults
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

        // ctx:int(message, key, opts?)
        methods.add_method_mut("int", |_, this, (message, key, opts): (String, String, Option<Table>)| {
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

            if let Some(ContextValue::Integer(_)) = this.data.get(&key) {
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

        // ctx:confirm(message, key, opts?)
        methods.add_method_mut("confirm", |_, this, (message, key, opts): (String, String, Option<Table>)| {
            let mut info = BoolPromptInfo::new(&message, Some(&key));

            if let Some(ref opts) = opts {
                info.default = get_opt_bool(opts, "default")?;
                info.help = get_opt_string(opts, "help")?;
                info.placeholder = get_opt_string(opts, "placeholder")?;
                if let Some(optional) = get_opt_bool(opts, "optional")? {
                    info.optional = optional;
                }
            }

            if let Some(ContextValue::Boolean(_)) = this.data.get(&key) {
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

        // ctx:select(message, key, options, opts?)
        methods.add_method_mut("select", |_, this, (message, key, options, opts): (String, String, Vec<String>, Option<Table>)| {
            let mut info = SelectPromptInfo::new(&message, Some(&key), options);

            if let Some(ref opts) = opts {
                info.default = get_opt_string(opts, "default")?;
                info.help = get_opt_string(opts, "help")?;
                info.placeholder = get_opt_string(opts, "placeholder")?;
                if let Some(optional) = get_opt_bool(opts, "optional")? {
                    info.optional = optional;
                }
            }

            if let Some(ContextValue::String(_)) = this.data.get(&key) {
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

            let response = this.send_prompt(ScriptMessage::PromptForSelect(info))?;
            if let Some(value) = handle_response_string(response)? {
                this.data.insert(key, ContextValue::String(value));
            }
            Ok(())
        });

        // ctx:multi_select(message, key, options, opts?)
        methods.add_method_mut("multi_select", |_, this, (message, key, options, opts): (String, String, Vec<String>, Option<Table>)| {
            let mut info = MultiSelectPromptInfo::new(&message, Some(&key), options);

            if let Some(ref opts) = opts {
                info.help = get_opt_string(opts, "help")?;
                info.placeholder = get_opt_string(opts, "placeholder")?;
                info.min_items = get_opt_i64(opts, "min")?.map(|v| v as usize);
                info.max_items = get_opt_i64(opts, "max")?.map(|v| v as usize);
                if let Some(optional) = get_opt_bool(opts, "optional")? {
                    info.optional = optional;
                }
            }

            if let Some(ContextValue::Array(_)) = this.data.get(&key) {
                return Ok(());
            }

            if this.use_default(&key) {
                if info.optional {
                    return Ok(());
                }
                return Err(LuaError::RuntimeError(format!(
                    "Headless mode: no answer or default for '{}'", message
                )));
            }

            let response = this.send_prompt(ScriptMessage::PromptForMultiSelect(info))?;
            if let Some(value) = handle_response_array(response)? {
                this.data.insert(key, ContextValue::Array(value));
            }
            Ok(())
        });

        // ctx:list(message, key, opts?)
        methods.add_method_mut("list", |_, this, (message, key, opts): (String, String, Option<Table>)| {
            let mut info = ListPromptInfo::new(&message, Some(&key));

            if let Some(ref opts) = opts {
                info.help = get_opt_string(opts, "help")?;
                info.placeholder = get_opt_string(opts, "placeholder")?;
                info.min_items = get_opt_i64(opts, "min")?.map(|v| v as usize);
                info.max_items = get_opt_i64(opts, "max")?.map(|v| v as usize);
                if let Some(optional) = get_opt_bool(opts, "optional")? {
                    info.optional = optional;
                }
            }

            if let Some(ContextValue::Array(_)) = this.data.get(&key) {
                return Ok(());
            }

            if this.use_default(&key) {
                if info.optional {
                    return Ok(());
                }
                return Err(LuaError::RuntimeError(format!(
                    "Headless mode: no answer or default for '{}'", message
                )));
            }

            let response = this.send_prompt(ScriptMessage::PromptForList(info))?;
            if let Some(value) = handle_response_array(response)? {
                this.data.insert(key, ContextValue::Array(value));
            }
            Ok(())
        });

        // ctx:editor(message, key, opts?)
        methods.add_method_mut("editor", |_, this, (message, key, opts): (String, String, Option<Table>)| {
            let mut info = EditorPromptInfo::new(&message, Some(&key));

            if let Some(ref opts) = opts {
                info.default = get_opt_string(opts, "default")?;
                info.help = get_opt_string(opts, "help")?;
                info.placeholder = get_opt_string(opts, "placeholder")?;
            }

            if let Some(ContextValue::String(_)) = this.data.get(&key) {
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
