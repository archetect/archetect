mod confirm;
mod editor;
mod int;
mod list;
mod multiselect;
mod select;
mod text;

use inquire::error::InquireResult;
use inquire::InquireError;
use rhai::plugin::*;
use rhai::{exported_module, Dynamic, Engine, EvalAltResult, Map};

use crate::errors::{ArchetectError, ArchetypeError};
use crate::archetype::archetype::Archetype;
use crate::archetype::archetype_context::ArchetypeContext;
use crate::runtime::context::RuntimeContext;
use crate::script::rhai::modules::cases::expand_cases;

pub(crate) fn register(
    engine: &mut Engine,
    archetype: Archetype,
    archetype_context: ArchetypeContext,
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
    engine.register_fn(
        "prompt",
        move |call: NativeCallContext, message: &str, settings: Map| {
            prompt_to_value(call, message, rt.clone(), settings)
        },
    );

    let rt = runtime_context.clone();
    engine.register_fn("prompt", move |call: NativeCallContext, message: &str| {
        prompt_to_value(call, message, rt.clone(), Map::new())
    });
}

fn prompt_to_map(
    call: NativeCallContext,
    _archetype: Archetype,
    archetype_context: ArchetypeContext,
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

    match prompt_type {
        PromptType::Text => {
            let value = text::prompt(call, message, &settings, &runtime_context, Some(key), answer)?;
            results.insert(key.into(), value.clone().into());
            expand_cases(&settings, &mut results, key, &value);
            return Ok(results.into());
        }
        PromptType::Confirm => {
            let value = confirm::prompt(message, &runtime_context, &settings, Some(key), answer)?;
            results.insert(key.into(), value.into());
            return Ok(results.into());
        }
        PromptType::Int => {
            let value = int::prompt(call, message, &runtime_context, &settings, Some(key), answer)?;
            results.insert(key.into(), value.into());
            return Ok(results.into());
        }
        PromptType::Select(options) => {
            let value = select::prompt(call, message, options, &runtime_context, &settings, Some(key), answer)?;
            results.insert(key.into(), value.clone().into());
            expand_cases(&settings, &mut results, key, &value);
            return Ok(results.into());
        }
        PromptType::MultiSelect(options) => {
            let value = multiselect::prompt(call, message, options, &runtime_context, &settings, Some(key), answer)?;
            results.insert(key.into(), value.into());
            return Ok(results.into());
        }
        PromptType::Editor => {
            let value = editor::prompt(message)?;
            results.insert(key.into(), value.into());
            return Ok(results.into());
        }
        PromptType::List => {
            let value = list::prompt(call, message, &settings, &runtime_context, Some(key), answer)?;
            results.insert(key.into(), value.clone().into());
            // TODO Consider casing strategies
            // expand_cases(&settings, &mut results, key, &value);
            return Ok(results.into());
        }
    }
}

fn prompt_to_value(
    call: NativeCallContext,
    message: &str,
    runtime_context: RuntimeContext,
    settings: Map,
) -> Result<Dynamic, Box<EvalAltResult>> {
    let prompt_type = get_prompt_type(&settings).map_err(|err| {
        Box::new(EvalAltResult::ErrorSystem(
            "Invalid PromptType".to_owned(),
            Box::new(err),
        ))
    })?;

    match prompt_type {
        PromptType::Text => {
            let value = text::prompt(call, message, &settings, &runtime_context, None, None)?;
            Ok(value.into())
        }
        PromptType::Confirm => {
            let value = confirm::prompt(message, &runtime_context, &settings, None, None)?;
            Ok(value.into())
        }
        PromptType::Int => {
            let value = int::prompt(call, message, &runtime_context, &settings, None, None)?;
            Ok(value.into())
        }
        PromptType::Select(options) => {
            let value = select::prompt(call, message, options, &runtime_context, &settings, None, None)?;
            Ok(value.into())
        }
        PromptType::MultiSelect(options) => {
            let value = multiselect::prompt(call, message, options, &runtime_context, &settings, None, None)?;
            Ok(value.into())
        }
        _ => panic!("Unimplemented PromptType"),
    }
}

fn get_answers(
    call: &NativeCallContext,
    settings: &Map,
    archetype_context: &ArchetypeContext,
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

fn handle_result<T>(result: InquireResult<T>) -> Result<T, Box<EvalAltResult>> {
    match result {
        Ok(value) => Ok(value),
        Err(err) => match err {
            InquireError::OperationCanceled => {
                return Err(Box::new(EvalAltResult::ErrorSystem(
                    "Cancelled".to_owned(),
                    Box::new(ArchetypeError::ValueRequired),
                )));
            }
            InquireError::OperationInterrupted => {
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
