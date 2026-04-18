use std::cell::RefCell;
use std::collections::BTreeMap;

use mlua::{Error as LuaError, Lua, Result as LuaResult, Table, UserData, UserDataMethods, Value};

use archetect_api::{
    BoolPromptInfo, ClientMessage, ContextMap, ContextValue, EditorPromptInfo, IntPromptInfo,
    ListPromptInfo, MultiSelectPromptInfo, ScriptMessage, SelectPromptInfo, TextPromptInfo,
};

use crate::archetype::render_context::RenderContext;
use crate::script::lua::cases::{CaseSpec, CaseSpecEntry, CaseSpecList};
use crate::Archetect;

/// Wrapper around the context's `BTreeMap` that bumps a version counter on
/// every mutation. Read access is via `Deref<Target = BTreeMap>`. Phase 8.1:
/// the version drives the `Context::lua_table_cache` invalidation.
#[derive(Clone, Debug, Default)]
struct ContextData {
    map: BTreeMap<String, ContextValue>,
    version: u64,
}

impl ContextData {
    fn insert(&mut self, key: String, value: ContextValue) -> Option<ContextValue> {
        self.version = self.version.wrapping_add(1);
        self.map.insert(key, value)
    }

    /// Mark the data as mutated without inserting. Use when an external helper
    /// is given `&mut BTreeMap` access via `map_mut` — call this before/after
    /// the mutation to invalidate downstream caches.
    fn bump_version(&mut self) {
        self.version = self.version.wrapping_add(1);
    }

    fn version(&self) -> u64 {
        self.version
    }

    /// Mutable map access. The version is bumped preemptively because the
    /// caller is expected to mutate. Used by `merge_into`-style helpers.
    fn map_mut(&mut self) -> &mut BTreeMap<String, ContextValue> {
        self.bump_version();
        &mut self.map
    }

    fn as_map(&self) -> &BTreeMap<String, ContextValue> {
        &self.map
    }
}

impl std::ops::Deref for ContextData {
    type Target = BTreeMap<String, ContextValue>;
    fn deref(&self) -> &Self::Target {
        &self.map
    }
}

pub struct Context {
    data: ContextData,
    archetect: Archetect,
    render_context: RenderContext,
    /// Cached Lua table built from `data`, keyed by the data's version. A
    /// fresh build happens on the first `to_lua_table` call after any
    /// mutation that bumps the version. Reads within an unchanged Context
    /// reuse the same table — large archetypes that render many files see
    /// the per-render build cost paid once.
    lua_table_cache: RefCell<Option<(u64, Table)>>,
}

impl Clone for Context {
    fn clone(&self) -> Self {
        Self {
            data: self.data.clone(),
            archetect: self.archetect.clone(),
            render_context: self.render_context.clone(),
            // Drop the cache on clone. Cheap to rebuild, and avoids any
            // ambiguity about which Lua state the cached table belongs to.
            lua_table_cache: RefCell::new(None),
        }
    }
}

impl std::fmt::Debug for Context {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Context")
            .field("data", &self.data)
            .field("archetect", &self.archetect)
            .field("render_context", &self.render_context)
            .finish_non_exhaustive()
    }
}

impl Context {
    pub fn new(archetect: Archetect, render_context: RenderContext) -> Self {
        // Pre-load answers from render context (now ContextMap)
        let mut data = ContextData::default();
        for (key, value) in render_context.answers() {
            data.insert(key.clone(), value.clone());
        }

        Self {
            data,
            archetect,
            render_context,
            lua_table_cache: RefCell::new(None),
        }
    }

    /// Convert context data to a ContextMap.
    pub fn to_context_map(&self) -> ContextMap {
        self.data.as_map().clone()
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
                CaseSpec::Input { key: input_key } => {
                    self.data.insert(input_key.clone(), ContextValue::String(value.to_string()));
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
    ///
    /// Phase 8.1: the result is cached and reused across calls until the next
    /// mutation. Templates rendered from a stable Context (the common case
    /// inside `directory.render`) see the per-render build cost paid once.
    pub fn to_lua_table(&self, lua: &Lua) -> LuaResult<Table> {
        let current_version = self.data.version();
        {
            let cache = self.lua_table_cache.borrow();
            if let Some((cached_version, table)) = cache.as_ref() {
                if *cached_version == current_version {
                    return Ok(table.clone());
                }
            }
        }
        let table = lua.create_table()?;
        for (key, value) in self.data.iter() {
            let lua_value = context_value_to_lua(lua, value)?;
            table.set(key.as_str(), lua_value)?;
        }
        *self.lua_table_cache.borrow_mut() = Some((current_version, table.clone()));
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

/// Deep-merge one ContextValue into a map under `key`.
///
/// Map∪Map recurses; any other combination replaces. This keeps
/// composition of namespaced contribution bags (e.g., `components.xtask`,
/// `components.tracing`) from clobbering each other at the parent level.
fn merge_into(dest: &mut BTreeMap<String, ContextValue>, key: String, incoming: ContextValue) {
    match (dest.get_mut(&key), incoming) {
        (Some(ContextValue::Map(existing)), ContextValue::Map(new_map)) => {
            for (k, v) in new_map {
                merge_context_map(existing, k, v);
            }
        }
        (_, incoming) => {
            dest.insert(key, incoming);
        }
    }
}

/// Same as `merge_into` but operating on a `ContextMap` reference directly.
fn merge_context_map(dest: &mut ContextMap, key: String, incoming: ContextValue) {
    match (dest.get_mut(&key), incoming) {
        (Some(ContextValue::Map(existing)), ContextValue::Map(new_map)) => {
            for (k, v) in new_map {
                merge_context_map(existing, k, v);
            }
        }
        (_, incoming) => {
            dest.insert(key, incoming);
        }
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

/// Shared body for `prompt_multiselect` and its deprecated alias
/// `prompt_multi_select`. Kept as a free function so both method
/// registrations can hand it directly to `add_method_mut`.
fn multiselect_prompt(
    _lua: &mlua::Lua,
    this: &mut Context,
    (message, key, options, opts): (String, String, Vec<String>, Option<Table>),
) -> LuaResult<Option<Vec<String>>> {
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
                let strings: Vec<String> = arr
                    .iter()
                    .filter_map(|v| match v {
                        ContextValue::String(s) => Some(s.clone()),
                        _ => None,
                    })
                    .collect();
                if answer_key != key {
                    this.data.insert(key, ContextValue::Array(arr));
                }
                return Ok(Some(strings));
            }
            ContextValue::String(s) => {
                let strings: Vec<String> = s.split(',').map(|s| s.trim().to_string()).collect();
                let items: Vec<ContextValue> =
                    strings.iter().cloned().map(ContextValue::String).collect();
                this.data.insert(key, ContextValue::Array(items));
                return Ok(Some(strings));
            }
            _ => {}
        }
    }

    if this.use_default(&key) {
        if let Some(ref defaults) = info.defaults {
            let arr: Vec<ContextValue> = defaults
                .iter()
                .cloned()
                .map(ContextValue::String)
                .collect();
            this.data.insert(key, ContextValue::Array(arr));
            return Ok(Some(defaults.clone()));
        }
        if info.optional {
            return Ok(None);
        }
        return Err(LuaError::RuntimeError(format!(
            "Headless mode: no answer or default for '{}'", message
        )));
    }

    let response = this.send_prompt(ScriptMessage::PromptForMultiSelect(info))?;
    if let Some(value) = handle_response_array(response)? {
        let arr: Vec<ContextValue> =
            value.iter().cloned().map(ContextValue::String).collect();
        this.data.insert(key, ContextValue::Array(arr));
        Ok(Some(value))
    } else {
        Ok(None)
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
        // tostring(ctx) — yields YAML of the context's data. Round-trips
        // as a valid archetect answer file, so `log.debug(tostring(ctx))`
        // doubles as "what answers would reproduce this render".
        // Equivalent to format.to_yaml(ctx).
        methods.add_meta_method("__tostring", |_, this, ()| -> LuaResult<String> {
            let context_map = this.to_context_map();
            let mut json_map = serde_json::Map::new();
            for (k, v) in &context_map {
                let json_val: serde_json::Value = v.clone().into();
                json_map.insert(k.clone(), json_val);
            }
            serde_yaml::to_string(&serde_json::Value::Object(json_map))
                .map_err(|e| LuaError::RuntimeError(format!("YAML serialization error: {}", e)))
        });

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

        // ctx:merge(value) — deep-merge another Context or a Lua table into this one.
        //
        // Map ∪ Map  → recursive merge (incoming keys win on leaf conflicts).
        // Everything else → replace.
        //
        // This is how components compose into a parent archetype's context.
        // Parents typically do:  context:merge(catalog.render("xtask", context))
        methods.add_method_mut("merge", |_, this, value: Value| {
            let incoming: ContextMap = match value {
                Value::UserData(ud) => {
                    if let Ok(other) = ud.borrow::<Context>() {
                        other.data.as_map().clone()
                    } else {
                        return Err(LuaError::RuntimeError(
                            "context:merge() expected a Context or table".to_string(),
                        ));
                    }
                }
                Value::Table(table) => match lua_table_to_context_value(&table)? {
                    ContextValue::Map(m) => m,
                    _ => {
                        return Err(LuaError::RuntimeError(
                            "context:merge() table must be a map (string keys), not an array".to_string(),
                        ));
                    }
                },
                Value::Nil => return Ok(()),
                _ => {
                    return Err(LuaError::RuntimeError(
                        "context:merge() expected a Context or table".to_string(),
                    ));
                }
            };
            for (k, v) in incoming {
                merge_into(this.data.map_mut(), k, v);
            }
            Ok(())
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
        // Returns the user's raw input as a string (or nil if the prompt
        // was skipped). Cases, when supplied, still expand into
        // context-side-effect keys — the return value remains the
        // single user-typed value.
        methods.add_method_mut("prompt_text", |_, this, (message, key, opts): (String, String, Option<Table>)| -> LuaResult<Option<String>> {
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
                return Ok(Some(answer));
            }

            if this.use_default(&key) {
                if let Some(ref default) = info.default {
                    this.store_string_with_cases(&key, default, &cases);
                    return Ok(Some(default.clone()));
                }
                if info.optional {
                    return Ok(None);
                }
                return Err(LuaError::RuntimeError(format!(
                    "Headless mode: no answer or default for '{}'", message
                )));
            }

            let response = this.send_prompt(ScriptMessage::PromptForText(info))?;
            if let Some(value) = handle_response_string(response)? {
                this.store_string_with_cases(&key, &value, &cases);
                Ok(Some(value))
            } else {
                Ok(None)
            }
        });

        // ctx:prompt_int(message, key, opts?) — returns the int (or nil).
        methods.add_method_mut("prompt_int", |_, this, (message, key, opts): (String, String, Option<Table>)| -> LuaResult<Option<i64>> {
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
                return Ok(Some(v));
            }

            if this.use_default(&key) {
                if let Some(default) = info.default {
                    this.data.insert(key, ContextValue::Integer(default));
                    return Ok(Some(default));
                }
                if info.optional {
                    return Ok(None);
                }
                return Err(LuaError::RuntimeError(format!(
                    "Headless mode: no answer or default for '{}'", message
                )));
            }

            let response = this.send_prompt(ScriptMessage::PromptForInt(info))?;
            if let Some(value) = handle_response_int(response)? {
                this.data.insert(key, ContextValue::Integer(value));
                Ok(Some(value))
            } else {
                Ok(None)
            }
        });

        // ctx:prompt_confirm(message, key, opts?) — returns the bool (or nil).
        methods.add_method_mut("prompt_confirm", |_, this, (message, key, opts): (String, String, Option<Table>)| -> LuaResult<Option<bool>> {
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
                return Ok(Some(v));
            }

            if this.use_default(&key) {
                if let Some(default) = info.default {
                    this.data.insert(key, ContextValue::Boolean(default));
                    return Ok(Some(default));
                }
                if info.optional {
                    return Ok(None);
                }
                return Err(LuaError::RuntimeError(format!(
                    "Headless mode: no answer or default for '{}'", message
                )));
            }

            let response = this.send_prompt(ScriptMessage::PromptForBool(info))?;
            if let Some(value) = handle_response_bool(response)? {
                this.data.insert(key, ContextValue::Boolean(value));
                Ok(Some(value))
            } else {
                Ok(None)
            }
        });

        // ctx:prompt_select(...) — returns the selected string (or nil).
        methods.add_method_mut("prompt_select", |_, this, (message, key, options, opts): (String, String, Vec<String>, Option<Table>)| -> LuaResult<Option<String>> {
            let mut info = SelectPromptInfo::new(&message, Some(&key), options);
            let cases = extract_cases(&opts);

            if let Some(ref opts) = opts {
                info.default = get_opt_string(opts, "default")?;
                info.help = get_opt_string(opts, "help")?;
                info.placeholder = get_opt_string(opts, "placeholder")?;
                if let Some(optional) = get_opt_bool(opts, "optional")? {
                    info.optional = optional;
                }
                if let Some(allow_other) = get_opt_bool(opts, "allow_other")? {
                    info.allow_other = allow_other;
                }
                info.other_label = get_opt_string(opts, "other_label")?;
            }

            let answer_key = get_answer_key(&opts, &key);
            if let Some(ContextValue::String(v)) = this.data.get(&answer_key).cloned() {
                this.store_string_with_cases(&key, &v, &cases);
                return Ok(Some(v));
            }

            if this.use_default(&key) {
                if let Some(ref default) = info.default {
                    this.store_string_with_cases(&key, default, &cases);
                    return Ok(Some(default.clone()));
                }
                if info.optional {
                    return Ok(None);
                }
                return Err(LuaError::RuntimeError(format!(
                    "Headless mode: no answer or default for '{}'", message
                )));
            }

            let response = this.send_prompt(ScriptMessage::PromptForSelect(info))?;
            if let Some(value) = handle_response_string(response)? {
                this.store_string_with_cases(&key, &value, &cases);
                Ok(Some(value))
            } else {
                Ok(None)
            }
        });

        // ctx:prompt_multiselect(message, key, options, opts?)
        //
        // Canonical name — matches the single-word suffix convention used by
        // the other prompt methods (prompt_text, prompt_int, prompt_select,
        // prompt_list, prompt_confirm, prompt_editor).
        methods.add_method_mut("prompt_multiselect", multiselect_prompt);

        // ctx:prompt_multi_select(...) — deprecated alias, logs a warning.
        //
        // Kept so archetypes written before the rename keep working. Remove
        // in a future version once the ecosystem has had time to migrate.
        methods.add_method_mut("prompt_multi_select", |lua, this, args: (String, String, Vec<String>, Option<Table>)| {
            let _ = this.archetect.request(ScriptMessage::LogWarn(
                "prompt_multi_select is deprecated; use prompt_multiselect instead.".to_string(),
            ));
            multiselect_prompt(lua, this, args)
        });

        // ctx:prompt_list(...) — returns the list of strings (or nil).
        methods.add_method_mut("prompt_list", |_, this, (message, key, opts): (String, String, Option<Table>)| -> LuaResult<Option<Vec<String>>> {
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
                        let strings: Vec<String> = arr
                            .iter()
                            .filter_map(|v| match v {
                                ContextValue::String(s) => Some(s.clone()),
                                _ => None,
                            })
                            .collect();
                        if answer_key != key {
                            this.data.insert(key, ContextValue::Array(arr));
                        }
                        return Ok(Some(strings));
                    }
                    ContextValue::String(s) => {
                        let strings: Vec<String> =
                            s.split(',').map(|s| s.trim().to_string()).collect();
                        let items: Vec<ContextValue> =
                            strings.iter().cloned().map(ContextValue::String).collect();
                        this.data.insert(key, ContextValue::Array(items));
                        return Ok(Some(strings));
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
                    return Ok(Some(defaults.clone()));
                }
                if info.optional {
                    return Ok(None);
                }
                return Err(LuaError::RuntimeError(format!(
                    "Headless mode: no answer or default for '{}'", message
                )));
            }

            let response = this.send_prompt(ScriptMessage::PromptForList(info))?;
            if let Some(value) = handle_response_array(response)? {
                let arr: Vec<ContextValue> =
                    value.iter().cloned().map(ContextValue::String).collect();
                this.data.insert(key, ContextValue::Array(arr));
                Ok(Some(value))
            } else {
                Ok(None)
            }
        });

        // ctx:prompt_editor(...) — returns the captured string (or nil).
        methods.add_method_mut("prompt_editor", |_, this, (message, key, opts): (String, String, Option<Table>)| -> LuaResult<Option<String>> {
            let mut info = EditorPromptInfo::new(&message, Some(&key));

            if let Some(ref opts) = opts {
                info.default = get_opt_string(opts, "default")?;
                info.help = get_opt_string(opts, "help")?;
                info.placeholder = get_opt_string(opts, "placeholder")?;
            }

            let answer_key = get_answer_key(&opts, &key);
            if let Some(ContextValue::String(v)) = this.data.get(&answer_key).cloned() {
                if answer_key != key {
                    this.data.insert(key, ContextValue::String(v.clone()));
                }
                return Ok(Some(v));
            }

            if this.use_default(&key) {
                if let Some(ref default) = info.default {
                    this.data.insert(key, ContextValue::String(default.clone()));
                    return Ok(Some(default.clone()));
                }
                if info.optional {
                    return Ok(None);
                }
                return Err(LuaError::RuntimeError(format!(
                    "Headless mode: no answer or default for '{}'", message
                )));
            }

            let response = this.send_prompt(ScriptMessage::PromptForEditor(info))?;
            if let Some(value) = handle_response_string(response)? {
                this.data.insert(key, ContextValue::String(value.clone()));
                Ok(Some(value))
            } else {
                Ok(None)
            }
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
    fn test_context_lua_table_cache_hit_returns_same_handle() {
        // Phase 8.1: two consecutive to_lua_table calls without intervening
        // mutations should return the same underlying Lua table handle.
        let lua = Lua::new();
        let mut context = make_context();
        context
            .data
            .insert("name".to_string(), ContextValue::String("foo".to_string()));

        let t1 = context.to_lua_table(&lua).unwrap();
        let t2 = context.to_lua_table(&lua).unwrap();

        // mlua tables are reference-counted handles. Two references to the
        // same Lua table compare equal via raw equality on the underlying
        // value. We assert by mutating one and observing the change in the
        // other — proof they reference the same table.
        t1.set("probe", "x").unwrap();
        let probe: String = t2.get("probe").unwrap();
        assert_eq!(probe, "x");
    }

    #[test]
    fn test_context_lua_table_cache_invalidates_on_set() {
        // Phase 8.1: a mutation between to_lua_table calls invalidates the
        // cache, so the second call observes the new key.
        let lua = Lua::new();
        let mut context = make_context();
        context
            .data
            .insert("a".to_string(), ContextValue::String("1".to_string()));

        let t1 = context.to_lua_table(&lua).unwrap();
        // Mutate via the wrapper's insert (bumps version).
        context
            .data
            .insert("b".to_string(), ContextValue::String("2".to_string()));
        let t2 = context.to_lua_table(&lua).unwrap();

        // The new table sees both keys.
        let a: String = t2.get("a").unwrap();
        let b: String = t2.get("b").unwrap();
        assert_eq!(a, "1");
        assert_eq!(b, "2");

        // The first table predates the mutation, so it does NOT see "b".
        let b_before: Value = t1.get("b").unwrap();
        assert!(
            matches!(b_before, Value::Nil),
            "expected nil for `b` in pre-mutation table, got {:?}",
            b_before
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
