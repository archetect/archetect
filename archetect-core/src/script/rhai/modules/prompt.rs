use std::borrow::Cow;
use std::str::FromStr;

use rhai::{Dynamic, Engine, EvalAltResult, exported_module, Map};
use rhai::plugin::*;

use inquire::error::InquireResult;
use inquire::InquireError;
use inquire::ui::{Color, RenderConfig, Styled};

use crate::archetype::render_context::RenderContext;
use crate::errors::{ArchetypeError, ArchetypeScriptError, ArchetypeScriptErrorWrapper};
use crate::runtime::context::RuntimeContext;
use crate::script::rhai::modules::cases::{CaseStyle, expand_key_value_cases};

mod bool;
mod editor;
mod int;
mod list;
mod multiselect;
mod select;
mod text;

pub(crate) fn register(
    engine: &mut Engine,
    archetype_context: RenderContext,
    runtime_context: RuntimeContext,
) {
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

fn prompt_to_map<'a, K: Into<Cow<'a, str>>>(
    call: &NativeCallContext,
    message: &str,
    runtime_context: RuntimeContext,
    render_context: RenderContext,
    key: K,
    settings: Map,
) -> Result<Dynamic, Box<EvalAltResult>> {
    let key = key.into();

    let prompt_type = cast_setting("type", &settings, message, None::<Cow<'_, str>>)
        .map_err(|err| ArchetypeScriptErrorWrapper(call, err))
        ?.unwrap_or_default()
        ;

    let mut results: Map = Map::new();

    let answers = &get_answers(call, message, &settings, Some(key.clone()), &render_context)?;
    let answer = answers.get(key.as_ref());

    return match prompt_type {
        PromptType::Text => {
            let value = text::prompt(call, message, &settings, &runtime_context, Some(key.clone()), answer)?;
            results.insert(key.as_ref().into(), value.clone().into());
            expand_key_value_cases(&settings, &mut results, key.as_ref(), value.to_string().as_str());
            Ok(results.into())
        }
        PromptType::Bool => {
            let value = bool::prompt(call, message, &runtime_context, &settings, Some(key.clone()), answer)?;
            results.insert(key.into(), value.into());
            Ok(results.into())
        }
        PromptType::Int => {
            let value = int::prompt_int(call, message, &runtime_context, &settings, Some(key.clone()), answer)?;
            results.insert(key.into(), value.into());
            Ok(results.into())
        }
        PromptType::Select(options) => {
            let value = select::prompt(
                call,
                message,
                options,
                &runtime_context,
                &settings,
                Some(key.clone()),
                answer,
            )?;
            results.insert(key.clone().into(), value.clone().into());
            expand_key_value_cases(&settings, &mut results, key.as_ref(), value.to_string().as_str());
            Ok(results.into())
        }
        PromptType::MultiSelect(options) => {
            let value = multiselect::prompt(
                call,
                message,
                options,
                &runtime_context,
                &settings,
                Some(key.clone()),
                answer,
            )?;
            results.insert(key.into(), value.into());
            Ok(results.into())
        }
        PromptType::Editor => {
            let value = editor::prompt(message)?;
            results.insert(key.into(), value.into());
            Ok(results.into())
        }
        PromptType::List => {
            let value = list::prompt(call, message, &runtime_context, &settings, Some(key.clone()), answer)?;
            results.insert(key.into(), value.clone().into());
            // TODO Consider casing strategies
            // expand_cases(&settings, &mut results, key, &value);
            Ok(results.into())
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
        .map_err(|err| ArchetypeScriptErrorWrapper(call, err))
        ?.unwrap_or_default()
        ;

    let case = cast_setting("cased_as", &settings, message, None::<Cow<'_, str>>)
        .map_err(|err| ArchetypeScriptErrorWrapper(call, err))
    ?;

    let answers = &get_answers::<Cow<'_, str>>(call, message, &settings, None::<Cow<'_, str>>, &render_context)?;
    let answer_key: Option<String> = cast_setting("answer_key", &settings, message, None::<Cow<'_, str>>)
        .map_err(|err| ArchetypeScriptErrorWrapper(call, err))
        ?;
    let answer = if let Some(key) = &answer_key {
        answers.get(key.as_str())
    } else {
        None
    };

    match prompt_type {
        PromptType::Text => {
            let value = text::prompt(call, message, &settings, &runtime_context, answer_key, answer)?;
            Ok(apply_case(&value, case))
        }
        PromptType::Bool => {
            let value = bool::prompt(call, message, &runtime_context, &settings, answer_key, answer)?;
            Ok(apply_case(&value, case))
        }
        PromptType::Int => {
            let value = int::prompt_int(call, message, &runtime_context, &settings, answer_key, answer)?;
            Ok(apply_case(&value, case))
        }
        PromptType::Select(options) => {
            let value = select::prompt(call, message, options, &runtime_context, &settings, answer_key, answer)?;
            Ok(apply_case(&value, case))
        }
        PromptType::MultiSelect(options) => {
            let value = multiselect::prompt(call, message, options, &runtime_context, &settings, answer_key, answer)?;
            Ok(apply_case(&value, case))
        }
        PromptType::Editor => {
            let value = editor::prompt(message)?;
            Ok(apply_case(&value, case))
        }
        PromptType::List => {
            let value = list::prompt(call, message, &runtime_context, &settings, answer_key, answer)?;
            Ok(apply_case(&value, case))
        }
    }
}

fn apply_case(input: &Dynamic, case: Option<CaseStyle>) -> Dynamic {
    if let Some(case) = case {
        if input.is_array() {
            return input
                .clone()
                .into_array()
                .unwrap()
                .iter()
                .map(|v| case.to_case(v.to_string().as_str()))
                .collect::<Vec<String>>()
                .into();
        }
        if input.is_unit() {
            return input.clone();
        }
        case.to_case(input.to_string().as_str()).into()
    } else {
        input.clone()
    }
}

fn get_answers<'a, K: Into<Cow<'a, str>>>(
    call: &NativeCallContext,
    message: &str,
    settings: &Map,
    key: Option<K>,
    archetype_context: &RenderContext,
) -> Result<Map, Box<EvalAltResult>> {
    let setting = "answer_source";
    if let Some(answers) = settings.get(setting) {
        if let Some(answers) = answers.clone().try_cast::<Map>() {
            return Ok(answers);
        } else {
            if !answers.is_unit() {
                let requirement = "a 'map' (\"#{{ .. }}\") or Unit type (\"()\")".to_string();
                let error =
                    ArchetypeScriptError::invalid_setting(message, setting, requirement, key);
                return Err(ArchetypeScriptErrorWrapper(call, error).into());
            } else {
                return Ok(Map::new());
            }
        }
    }
    let answers = archetype_context.answers();
    let mut results = Map::new();
    results.extend(answers.clone());
    return Ok(results);
}

pub fn get_optional_setting(settings: &Map) -> bool {
    settings
        .get("optional")
        .map_or(Ok(false), |value| value.as_bool())
        .unwrap_or(false)
}

pub fn get_render_config() -> RenderConfig {
    RenderConfig::default_colored().with_canceled_prompt_indicator(Styled::new("<none>").with_fg(Color::DarkGrey))
}

pub fn parse_setting<T>(setting: &str, settings: &Map) -> Option<T>
where
    T: FromStr,
{
    settings
        .get(setting)
        .map(|value| value.to_string().parse::<T>())
        .map(|value| value.ok())
        .flatten()
}

pub fn cast_setting<'a, P, T, K>(setting: &str, settings: &Map, prompt: P, key: Option<K>) -> Result<Option<T>, ArchetypeScriptError>
where
    P: AsRef<str>,
    K: Into<Cow<'a, str>>,
    T: RequirementDescription + 'static
{
    match settings.get(setting) {
        None => return Ok(None),
        Some(value) => {
            if let Some(value) = value.clone().try_cast::<T>() {
                return Ok(Some(value));
            }
            return Err(ArchetypeScriptError::invalid_setting(
                prompt.as_ref(),
                setting,
                T::get_requirement(),
                key,
            ));
        }
    }
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

fn handle_result<T>(result: InquireResult<T>, optional: bool) -> Result<Dynamic, Box<EvalAltResult>>
where
    T: Into<Dynamic>,
{
    match result {
        Ok(value) => Ok(value.into()),
        Err(err) => match err {
            InquireError::OperationCanceled => {
                if optional {
                    return Ok(Dynamic::UNIT);
                }
                return Err(Box::new(EvalAltResult::ErrorSystem(
                    "Cancelled".to_owned(),
                    Box::new(ArchetypeError::ValueRequired),
                )));
            }
            InquireError::OperationInterrupted => {
                if optional {
                    return Ok(Dynamic::UNIT);
                }
                return Err(Box::new(EvalAltResult::ErrorSystem(
                    "Cancelled".to_owned(),
                    Box::new(ArchetypeError::OperationInterrupted),
                )));
            }
            err => Err(Box::new(EvalAltResult::ErrorSystem("Error".to_owned(), Box::new(err)))),
        },
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
