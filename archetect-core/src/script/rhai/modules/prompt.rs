use std::borrow::Cow;
use std::str::FromStr;

use rhai::{Dynamic, Engine, EvalAltResult, exported_module, Map};
use rhai::plugin::*;
use archetect_api::{PromptInfo, PromptInfoItemsRestrictions, PromptInfoLengthRestrictions, PromptInfoPageable};

use crate::archetype::render_context::RenderContext;
use crate::errors::{ArchetypeScriptError, ArchetypeScriptErrorWrapper};
use crate::runtime::context::RuntimeContext;
use crate::script::rhai::modules::cases::{CaseStyle, expand_key_value_cases};

mod bool;
mod editor;
mod int;
mod list;
mod multiselect;
mod select;
mod text;

pub(crate) fn register(engine: &mut Engine, archetype_context: RenderContext, runtime_context: RuntimeContext) {
    engine.register_global_module(exported_module!(module).into());

    let ctx = archetype_context.clone();
    let rt = runtime_context.clone();
    engine.register_fn(
        "prompt",
        move |call: NativeCallContext, message: &str, key: &str, settings: Map| {
            prompt_to_map(&call, message, rt.clone(), ctx.clone(), key, settings)
        },
    );

    let ctx = archetype_context.clone();
    let rt = runtime_context.clone();
    engine.register_fn("prompt", move |call: NativeCallContext, message: &str, key: &str| {
        prompt_to_map(&call, message, rt.clone(), ctx.clone(), key, Map::new())
    });

    let rt = runtime_context.clone();
    let ctx = archetype_context.clone();
    engine.register_fn(
        "prompt",
        move |call: NativeCallContext, message: &str, settings: Map| {
            prompt_to_value(&call, message, rt.clone(), ctx.clone(), settings)
        },
    );

    let rt = runtime_context.clone();
    let ctx = archetype_context.clone();
    engine.register_fn("prompt", move |call: NativeCallContext, message: &str| {
        prompt_to_value(&call, message, rt.clone(), ctx.clone(), Map::new())
    });
}

fn prompt_to_map<'a, K: AsRef<str>>(
    call: &NativeCallContext,
    message: &str,
    runtime_context: RuntimeContext,
    render_context: RenderContext,
    key: K,
    settings: Map,
) -> Result<Dynamic, Box<EvalAltResult>> {
    let key = key.as_ref();

    let prompt_type = cast_setting("type", &settings, message, Some(key))
        .map_err(|err| ArchetypeScriptErrorWrapper(call, err))?
        .unwrap_or_default();

    let mut results: Map = Map::new();

    let answers = &get_answers(call, message, &settings, Some(key), &render_context)?;
    let answer = answers.get(key);

    return match prompt_type {
        PromptType::Text => {
            let value = text::prompt(call, message, &settings, &runtime_context, Some(key), answer)?;
            match value {
                None => {
                    results.insert(key.into(), Dynamic::UNIT);
                    expand_key_value_cases(&settings, &mut results, key.as_ref(), Caseable::Opaque(Dynamic::UNIT));
                    Ok(results.into())
                },
                Some(value) => {
                    results.insert(key.into(), value.clone().into());
                    expand_key_value_cases(&settings, &mut results, key.as_ref(), Caseable::String(value));
                    Ok(results.into())
                }
            }

        }
        PromptType::Bool => {
            let value = bool::prompt(call, message, &runtime_context, &settings, Some(key), answer)?;
            match value {
                None => {
                    results.insert(key.into(), Dynamic::UNIT);
                    expand_key_value_cases(&settings, &mut results, key.as_ref(), Caseable::Opaque(Dynamic::UNIT));
                    Ok(results.into())
                },
                Some(value) => {
                    results.insert(key.into(), value.into());
                    expand_key_value_cases(&settings, &mut results, key.as_ref(), Caseable::Opaque(value.into()));
                    Ok(results.into())
                }
            }
        }
        PromptType::Int => {
            let value = int::prompt_int(call, message, &runtime_context, &settings, Some(key), answer)?;
            match value {
                None => {
                    results.insert(key.into(), Dynamic::UNIT);
                    expand_key_value_cases(&settings, &mut results, key.as_ref(), Caseable::Opaque(Dynamic::UNIT));
                    Ok(results.into())
                },
                Some(value) => {
                    results.insert(key.into(), value.into());
                    expand_key_value_cases(&settings, &mut results, key.as_ref(), Caseable::Opaque(value.into()));
                    Ok(results.into())
                }
            }
        }
        PromptType::Select(options) => {
            let value = select::prompt(
                call,
                message,
                options,
                &runtime_context,
                &settings,
                Some(key),
                answer,
            )?;
            match value {
                None => {
                    results.insert(key.into(), Dynamic::UNIT);
                    expand_key_value_cases(&settings, &mut results, key.as_ref(), Caseable::Opaque(Dynamic::UNIT));
                    Ok(results.into())
                },
                Some(value) => {
                    results.insert(key.into(), value.clone().into());
                    expand_key_value_cases(&settings, &mut results, key.as_ref(), Caseable::String(value));
                    Ok(results.into())
                }
            }
        }
        PromptType::MultiSelect(options) => {
            let value = multiselect::prompt(
                call,
                message,
                options,
                &runtime_context,
                &settings,
                Some(key),
                answer,
            )?;
            match value {
                None => {
                    results.insert(key.into(), Dynamic::UNIT);
                    expand_key_value_cases(&settings, &mut results, key.as_ref(), Caseable::Opaque(Dynamic::UNIT));
                    Ok(results.into())
                },
                Some(value) => {
                    let dynamic_list = value.clone().into_iter()
                        .map(|v|Dynamic::from(v))
                        .collect::<Vec<Dynamic>>();
                    results.insert(key.into(), dynamic_list.into());
                    expand_key_value_cases(&settings, &mut results, key.as_ref(), Caseable::List(value));
                    Ok(results.into())
                }
            }
        }
        PromptType::Editor => {
            let value = editor::prompt(call, message, &settings, &runtime_context, Some(key), answer)?;
            match value {
                None => {
                    results.insert(key.into(), Dynamic::UNIT);
                    expand_key_value_cases(&settings, &mut results, key.as_ref(), Caseable::Opaque(Dynamic::UNIT));
                    Ok(results.into())
                },
                Some(value) => {
                    results.insert(key.into(), value.clone().into());
                    expand_key_value_cases(&settings, &mut results, key.as_ref(), Caseable::String(value));
                    Ok(results.into())
                }
            }
        }
        PromptType::List => {
            match list::prompt(call, message, &runtime_context, &settings, Some(key), answer)? {
                None => {
                    results.insert(key.into(), Dynamic::UNIT);
                    expand_key_value_cases(&settings, &mut results, key.as_ref(), Caseable::Opaque(Dynamic::UNIT));
                    Ok(results.into())
                },
                Some(list) => {
                    let dynamic_list = list.clone().into_iter()
                        .map(|v|Dynamic::from(v))
                        .collect::<Vec<Dynamic>>();
                    results.insert(key.into(), dynamic_list.into());
                    expand_key_value_cases(&settings, &mut results, key, Caseable::List(list));
                    Ok(results.into())
                }
            }
        }
    };
}

fn prompt_to_value(
    call: &NativeCallContext,
    message: &str,
    runtime_context: RuntimeContext,
    render_context: RenderContext,
    settings: Map,
) -> Result<Dynamic, Box<EvalAltResult>> {
    let prompt_type = cast_setting("type", &settings, message, None::<Cow<'_, str>>)
        .map_err(|err| ArchetypeScriptErrorWrapper(call, err))?
        .unwrap_or_default();

    let case = cast_setting("cased_as", &settings, message, None::<Cow<'_, str>>)
        .map_err(|err| ArchetypeScriptErrorWrapper(call, err))?;

    let answers = &get_answers::<Cow<'_, str>>(call, message, &settings, None::<Cow<'_, str>>, &render_context)?;
    let answer_key: Option<String> = cast_setting("answer_key", &settings, message, None::<Cow<'_, str>>)
        .map_err(|err| ArchetypeScriptErrorWrapper(call, err))?;
    let answer = if let Some(key) = &answer_key {
        answers.get(key.as_str())
    } else {
        None
    };

    match prompt_type {
        PromptType::Text => {
            let value = text::prompt(call, message, &settings, &runtime_context,  answer_key.as_ref(), answer)?;
            match value {
                None => Ok(Dynamic::UNIT),
                Some(list) => Ok(apply_case(Caseable::String(list), case)),
            }
        }
        PromptType::Bool => {
            let value = bool::prompt(call, message, &runtime_context, &settings, answer_key, answer)?;
            match value {
                None => Ok(Dynamic::UNIT),
                Some(value) => Ok(value.into())
            }
        }
        PromptType::Int => {
            let value = int::prompt_int(call, message, &runtime_context, &settings, answer_key, answer)?;
            match value {
                None => Ok(Dynamic::UNIT),
                Some(value) => Ok(value.into())
            }
        }
        PromptType::Select(options) => {
            let value = select::prompt(call, message, options, &runtime_context, &settings, answer_key, answer)?;
            match value {
                Some(value) => Ok(apply_case(Caseable::String(value), case)),
                None => Ok(Dynamic::UNIT),
            }
        }
        PromptType::MultiSelect(options) => {
            let value = multiselect::prompt(call, message, options, &runtime_context, &settings, answer_key, answer)?;
            match value {
                Some(value) => Ok(apply_case(Caseable::List(value), case)),
                None => Ok(Dynamic::UNIT),
            }
        }
        PromptType::Editor => {
            let value = editor::prompt(call, message, &settings, &runtime_context,  answer_key, answer)?;
            match value {
                None => Ok(Dynamic::UNIT),
                Some(list) => Ok(apply_case(Caseable::String(list), case)),
            }
        }
        PromptType::List => {
            let value = list::prompt(call, message, &runtime_context, &settings, answer_key, answer)?;
            match value {
                None => Ok(Dynamic::UNIT),
                Some(list) => Ok(apply_case(Caseable::List(list), case)),
            }
        }
    }
}

fn apply_case(input: Caseable, case: Option<CaseStyle>) -> Dynamic {
    match case {
        None => match input {
            Caseable::String(value) => Dynamic::from(value),
            Caseable::List(value) => Dynamic::from(value),
            Caseable::Opaque(value) => value.clone_cast(),
        },
        Some(case) => match input.into() {
            Caseable::String(value) => Dynamic::from(case.to_case(&value)),
            Caseable::List(list) => {
                let result = list.into_iter()
                    .map(|v| case.to_case(&v))
                    .map(|v|Dynamic::from(v))
                    .collect::<Vec<Dynamic>>();
                Dynamic::from(result)
            }
            Caseable::Opaque(value) => value.clone_cast(),
        },
    }
}

fn get_answers<'a, K: AsRef<str> + Clone>(
    call: &NativeCallContext,
    message: &str,
    settings: &Map,
    key: Option<K>,
    archetype_context: &RenderContext,
) -> Result<Map, Box<EvalAltResult>> {
    let setting = "answer_source";
    if let Some(answers) = settings.get(setting) {
        return if let Some(answers) = answers.clone().try_cast::<Map>() {
            Ok(answers)
        } else {
            if !answers.is_unit() {
                let requirement = "a 'map' (\"#{{ .. }}\") or Unit type (\"()\")".to_string();
                let error = ArchetypeScriptError::invalid_setting(message, key, setting, requirement);
                Err(ArchetypeScriptErrorWrapper(call, error).into())
            } else {
                Ok(Map::new())
            }
        }
    }
    let answers = archetype_context.answers();
    let mut results = Map::new();
    results.extend(answers.clone());
    return Ok(results);
}

pub fn parse_setting<'a, P, T, K>(
    setting: &str,
    settings: &Map,
    prompt: P,
    key: Option<K>,
) -> Result<Option<T>, ArchetypeScriptError>
where
    P: AsRef<str>,
    K: AsRef<str>,
    T: RequirementDescription + FromStr + 'static,
{
    match settings.get(setting) {
        None => return Ok(None),
        Some(value) => {
            if let Ok(value) = value.to_string().parse::<T>() {
                return Ok(Some(value));
            }
            return Err(ArchetypeScriptError::invalid_setting(
                prompt.as_ref(),
                key.as_ref(),
                setting,
                T::get_requirement(),
            ));
        }
    }
}

pub fn cast_setting<'a, P, T, K>(
    setting: &str,
    settings: &Map,
    prompt: P,
    key: Option<K>,
) -> Result<Option<T>, ArchetypeScriptError>
where
    P: AsRef<str>,
    K: AsRef<str>,
    T: RequirementDescription + 'static,
{
    match settings.get(setting) {
        None => return Ok(None),
        Some(value) => {
            if let Some(value) = value.clone().try_cast::<T>() {
                return Ok(Some(value));
            }
            return Err(ArchetypeScriptError::invalid_setting(
                prompt.as_ref(),
                key.as_ref(),
                setting,
                T::get_requirement(),
            ));
        }
    }
}

pub fn extract_prompt_info<T: PromptInfo>(prompt_info: &mut T, settings: &Map) -> Result<(), ArchetypeScriptError> {
    let optional = cast_setting("optional", settings, prompt_info.message(), prompt_info.key())?
        .unwrap_or_default();
    let placeholder = cast_setting("placeholder", settings, prompt_info.message(), prompt_info.key())?;
    let help = cast_setting("help", settings, prompt_info.message(), prompt_info.key())?
        .or_else(|| if optional { Some("<esc> for None".to_string()) } else { None })
        ;
    prompt_info.set_optional(optional);
    prompt_info.set_placeholder(placeholder);
    prompt_info.set_help(help);
    Ok(())
}

pub fn extract_prompt_length_restrictions<T: PromptInfoLengthRestrictions>(prompt_info: &mut T, settings: &Map) -> Result<(), ArchetypeScriptError> {
    let min = parse_setting("min", settings, prompt_info.message(), prompt_info.key())?;
    let max = parse_setting("max", settings, prompt_info.message(), prompt_info.key())?;
    // Don't overwrite defaults, if set
    if min.is_some() {
        prompt_info.set_min(min);
    }
    // Don't overwrite defaults, if set
    if max.is_some() {
        prompt_info.set_max(max)
    }
    Ok(())
}

pub fn extract_prompt_items_restrictions<T: PromptInfoItemsRestrictions>(prompt_info: &mut T, settings: &Map) -> Result<(), ArchetypeScriptError> {
    let min_items = parse_setting("min_items", settings, prompt_info.message(), prompt_info.key())?;
    let max_items = parse_setting("max_items", settings, prompt_info.message(), prompt_info.key())?;
    // Don't overwrite defaults, if set
    if min_items.is_some() {
        prompt_info.set_min_items(min_items);
    }
    // Don't overwrite defaults, if set
    if max_items.is_some() {
        prompt_info.set_max_items(max_items);
    }
    Ok(())
}

pub fn extract_prompt_info_pageable<T: PromptInfoPageable>(prompt_info: &mut T, settings: &Map) -> Result<(), ArchetypeScriptError> {
    let page_size = parse_setting("page_size", settings, prompt_info.message(), prompt_info.key())?;
    // Don't overwrite defaults, if set
    if page_size.is_some() {
        prompt_info.set_page_size(page_size);
    }
    Ok(())
}

pub trait RequirementDescription {
    fn get_requirement() -> Cow<'static, str>;
}

impl RequirementDescription for String {
    fn get_requirement() -> Cow<'static, str> {
        "a String".into()
    }
}

impl RequirementDescription for PromptType {
    fn get_requirement() -> Cow<'static, str> {
        "one of Text, Bool, Int, List, Select, MultiSelect, or Editor".into()
    }
}

impl RequirementDescription for CaseStyle {
    fn get_requirement() -> Cow<'static, str> {
        "a CaseStyle".into()
    }
}

impl RequirementDescription for usize {
    fn get_requirement() -> Cow<'static, str> {
        "a positive Integer".into()
    }
}

impl RequirementDescription for i64 {
    fn get_requirement() -> Cow<'static, str> {
        "an Integer".into()
    }
}

impl RequirementDescription for bool {
    fn get_requirement() -> Cow<'static, str> {
        "a boolean".into()
    }
}

impl RequirementDescription for Map {
    fn get_requirement() -> Cow<'static, str> {
        "a Map".into()
    }
}

pub enum Caseable {
    String(String),
    List(Vec<String>),
    Opaque(Dynamic),
}

#[derive(Clone, Debug)]
pub enum PromptType {
    Text,
    Bool,
    Int,
    List,
    Select(Vec<Dynamic>),
    MultiSelect(Vec<Dynamic>),
    Editor,
}

impl Default for PromptType {
    fn default() -> Self {
        PromptType::Text
    }
}

#[allow(non_snake_case)]
#[allow(non_upper_case_globals)]
#[export_module]
pub mod module {
    use rhai::Dynamic;

    pub type PromptType = crate::script::rhai::modules::prompt::PromptType;

    pub const Text: PromptType = PromptType::Text;
    pub const String: PromptType = PromptType::Text;
    pub const Confirm: PromptType = PromptType::Bool;
    pub const Bool: PromptType = PromptType::Bool;
    pub const Int: PromptType = PromptType::Int;
    pub const List: PromptType = PromptType::List;
    pub const Editor: PromptType = PromptType::Editor;

    pub fn Select(options: Vec<Dynamic>) -> PromptType {
        PromptType::Select(options)
    }

    pub fn MultiSelect(options: Vec<Dynamic>) -> PromptType {
        PromptType::MultiSelect(options)
    }
}
