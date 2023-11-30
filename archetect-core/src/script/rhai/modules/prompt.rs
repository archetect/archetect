use std::str::FromStr;
use rhai::plugin::*;
use rhai::{exported_module, Dynamic, Engine, EvalAltResult, Map};

use inquire::error::InquireResult;
use inquire::ui::{Color, RenderConfig, Styled};
use inquire::InquireError;

use crate::archetype::archetype::Archetype;
use crate::archetype::render_context::RenderContext;
use crate::errors::{ArchetectError, ArchetypeError};
use crate::runtime::context::RuntimeContext;
use crate::script::rhai::modules::cases::{CaseStyle, expand_key_value_cases};

mod confirm;
mod editor;
mod int;
mod list;
mod multiselect;
mod select;
mod text;

pub(crate) fn register(
    engine: &mut Engine,
    archetype: Archetype,
    archetype_context: RenderContext,
    runtime_context: RuntimeContext,
) {
    engine.register_global_module(exported_module!(module).into());

    let arch = archetype.clone();
    let ctx = archetype_context.clone();
    let rt = runtime_context.clone();
    engine.register_fn(
        "prompt",
        move |call: NativeCallContext, message: &str, key: &str, settings: Map| {
            prompt_to_map(call, arch.clone(), ctx.clone(), rt.clone(), message, key, settings)
        },
    );

    let arch = archetype.clone();
    let ctx = archetype_context.clone();
    let rt = runtime_context.clone();
    engine.register_fn("prompt", move |call: NativeCallContext, message: &str, key: &str| {
        prompt_to_map(call, arch.clone(), ctx.clone(), rt.clone(), message, key, Map::new())
    });

    let rt = runtime_context.clone();
    let ctx = archetype_context.clone();
    engine.register_fn(
        "prompt",
        move |call: NativeCallContext, message: &str, settings: Map| {
            prompt_to_value(call, message, rt.clone(), ctx.clone(), settings)
        },
    );

    let rt = runtime_context.clone();
    let ctx = archetype_context.clone();
    engine.register_fn("prompt", move |call: NativeCallContext, message: &str| {
        prompt_to_value(call, message, rt.clone(), ctx.clone(), Map::new())
    });
}

fn prompt_to_map(
    call: NativeCallContext,
    _archetype: Archetype,
    archetype_context: RenderContext,
    runtime_context: RuntimeContext,
    message: &str,
    key: &str,
    settings: Map,
) -> Result<Dynamic, Box<EvalAltResult>> {
    let prompt_type = get_prompt_type(&settings).map_err(|err| {
        Box::new(EvalAltResult::ErrorSystem(
            "Invalid PromptType".to_owned(),
            Box::new(err),
        ))
    })?;

    let mut results: Map = Map::new();

    let answers = &get_answers(&call, &settings, &archetype_context)?;
    let answer = answers.get(key);

    return match prompt_type {
        PromptType::Text => {
            let value = text::prompt(call, message, &settings, &runtime_context, Some(key), answer)?;
            results.insert(key.into(), value.clone().into());
            expand_key_value_cases(&settings, &mut results, key, value.to_string().as_str());
            Ok(results.into())
        }
        PromptType::Confirm => {
            let value = confirm::prompt(message, &runtime_context, &settings, Some(key), answer)?;
            results.insert(key.into(), value.into());
            Ok(results.into())
        }
        PromptType::Int => {
            let value = int::prompt(call, message, &runtime_context, &settings, Some(key), answer)?;
            results.insert(key.into(), value.into());
            Ok(results.into())
        }
        PromptType::Select(options) => {
            let value = select::prompt(call, message, options, &runtime_context, &settings, Some(key), answer)?;
            results.insert(key.into(), value.clone().into());
            expand_key_value_cases(&settings, &mut results, key, value.to_string().as_str());
            Ok(results.into())
        }
        PromptType::MultiSelect(options) => {
            let value = multiselect::prompt(call, message, options, &runtime_context, &settings, Some(key), answer)?;
            results.insert(key.into(), value.into());
            Ok(results.into())
        }
        PromptType::Editor => {
            let value = editor::prompt(message)?;
            results.insert(key.into(), value.into());
            Ok(results.into())
        }
        PromptType::List => {
            let value = list::prompt(call, message, &runtime_context, &settings, Some(key), answer)?;
            results.insert(key.into(), value.clone().into());
            // TODO Consider casing strategies
            // expand_cases(&settings, &mut results, key, &value);
            Ok(results.into())
        }
    };
}

fn prompt_to_value(
    call: NativeCallContext,
    message: &str,
    runtime_context: RuntimeContext,
    archetype_context: RenderContext,
    settings: Map,
) -> Result<Dynamic, Box<EvalAltResult>> {
    let prompt_type = get_prompt_type(&settings).map_err(|err| {
        Box::new(EvalAltResult::ErrorSystem(
            "Invalid PromptType".to_owned(),
            Box::new(err),
        ))
    })?;

    let case = settings.get("cased_as")
        .map(|case| case.clone().try_cast::<CaseStyle>())
        .flatten();

    let answers = &get_answers(&call, &settings, &archetype_context)?;
    let answer_key = settings.get("answer_key").map(|value| value.to_string());
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
        PromptType::Confirm => {
            let value = confirm::prompt(message, &runtime_context, &settings, answer_key, answer)?;
            Ok(apply_case(&value, case))
        }
        PromptType::Int => {
            let value = int::prompt(call, message, &runtime_context, &settings, answer_key, answer)?;
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
            return input.clone().into_array().unwrap().iter()
                .map(|v| case.to_case(v.to_string().as_str()))
                .collect::<Vec<String>>().into();
        }
        if input.is_unit() {
            return input.clone();
        }
        case.to_case(input.to_string().as_str()).into()
    } else {
        input.clone()
    }
}

fn get_answers(
    call: &NativeCallContext,
    settings: &Map,
    archetype_context: &RenderContext,
) -> Result<Map, Box<EvalAltResult>> {
    if let Some(answers) = settings.get("answer_source") {
        if let Some(answers) = answers.clone().try_cast::<Map>() {
            return Ok(answers);
        } else {
            if !answers.is_unit() {
                let fn_name = call.fn_name().to_owned();
                let source = call.source().unwrap_or_default().to_owned();
                let position = call.position();
                let error = EvalAltResult::ErrorSystem(
                    "Invalid Configuration".to_owned(),
                    Box::new(ArchetectError::GeneralError(
                        format!(
                            "When specifying the 'answer_source' property, it must be a 'map' (\"#{{ .. }}\") or Unit type (\"()\"), \
                            but it was of type '{}'",
                            answers.type_name()
                        )
                        .to_owned(),
                    )),
                );
                return Err(Box::new(EvalAltResult::ErrorInFunctionCall(
                    fn_name,
                    source,
                    Box::new(error),
                    position,
                )));
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

pub fn get_prompt_type(settings: &Map) -> Result<PromptType, ArchetypeError> {
    if let Some(prompt_type) = settings.get("type") {
        if let Some(prompt_type) = prompt_type.clone().try_cast::<PromptType>() {
            // TODO: Throw Error if a value was provided but it is NOT a PromptType
            return Ok(prompt_type);
        }
    }
    Ok(PromptType::Text)
}

pub fn get_optional_setting(settings: &Map) -> bool {
    settings
        .get("optional")
        .map_or(Ok(false), |value| value.as_bool())
        .unwrap_or(false)
}

pub fn get_render_config() -> RenderConfig {
    RenderConfig::default_colored()
        .with_canceled_prompt_indicator(Styled::new("<none>").with_fg(Color::DarkGrey))
}

pub fn parse_setting<T>(setting: &str, settings: &Map) -> Option<T>
    where T: FromStr,
{
    settings
        .get(setting)
        .map(|value| value.to_string().parse::<T>())
        .map(|value| value.ok())
        .flatten()
}

#[derive(Clone, Debug)]
pub enum PromptType {
    Text,
    Confirm,
    Int,
    List,
    Select(Vec<Dynamic>),
    MultiSelect(Vec<Dynamic>),
    Editor,
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
    pub const Confirm: PromptType = PromptType::Confirm;
    pub const Bool: PromptType = PromptType::Confirm;
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

pub fn create_error_from_call(call: &NativeCallContext, message: &str, error: ArchetectError) -> Box<EvalAltResult> {
    let fn_name = call.fn_name().to_owned();
    let source = call.source().unwrap_or_default().to_owned();
    let position = call.position();
    let error = EvalAltResult::ErrorSystem(message.to_owned(), Box::new(error));
    return Box::new(EvalAltResult::ErrorInFunctionCall(
        fn_name,
        source,
        Box::new(error),
        position,
    ));
}
