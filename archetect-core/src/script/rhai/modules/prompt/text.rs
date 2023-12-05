use rhai::{Dynamic, EvalAltResult, Map, NativeCallContext};

use archetect_api::{CommandRequest, CommandResponse, PromptInfo, TextPromptInfo};
use archetect_api::validations::validate_text;

use crate::errors::{ArchetypeScriptError, ArchetypeScriptErrorWrapper};
use crate::runtime::context::RuntimeContext;
use crate::script::rhai::modules::prompt::{cast_setting, parse_setting};

pub fn prompt<'a, K: AsRef<str> + Clone>(
    call: &NativeCallContext,
    message: &str,
    settings: &Map,
    runtime_context: &RuntimeContext,
    key: Option<K>,
    answer: Option<&Dynamic>,
) -> Result<Option<String>, Box<EvalAltResult>> {
    let optional = cast_setting("optional", settings, message, key.clone())
        .map_err(|error| ArchetypeScriptErrorWrapper(call, error))?
        .unwrap_or_default();
    let min = parse_setting("min", settings, message, key.clone())
        .map_err(|error| ArchetypeScriptErrorWrapper(call, error))?
        .or(Some(1))
        ;
    let max = parse_setting("max", settings, message, key.clone())
        .map_err(|error| ArchetypeScriptErrorWrapper(call, error))?;
    let placeholder = cast_setting("placeholder", settings, message, key.clone())
        .map_err(|error| ArchetypeScriptErrorWrapper(call, error))?;
    let help = cast_setting("help", settings, message, key.clone())
        .map_err(|error| ArchetypeScriptErrorWrapper(call, error))?
        .or_else(|| if optional { Some("<esc> for None".to_string()) } else { None })
        ;
    let defaults_with = cast_setting("defaults_with", settings, message, key.clone())
        .map_err(|error| ArchetypeScriptErrorWrapper(call, error))?;

    let prompt_info = TextPromptInfo::new(message)
        .with_optional(optional)
        .with_min(min)
        .with_max(max)
        .with_placeholder(placeholder)
        .with_help(help)
        .with_default(defaults_with.clone())
        ;

    if let Some(answer) = answer {
        return if let Some(answer) = answer.clone().try_cast::<String>() {
            match validate_text(min, max, &answer.to_string()) {
                Ok(_) => Ok(answer.into()),
                Err(error_message) => {
                    let error = ArchetypeScriptError::answer_validation_error(answer.to_string(), message, key, error_message);
                    return Err(ArchetypeScriptErrorWrapper(call, error).into());
                }
            }
        } else {
            let error = ArchetypeScriptError::answer_type_error(answer.to_string(), message, key, "a String");
            Err(ArchetypeScriptErrorWrapper(call, error).into())
        }
    }

    if runtime_context.headless() {
        if let Some(default) = defaults_with {
            return Ok(Some(default));
        } else if optional {
            return Ok(None);
        }
        let error = ArchetypeScriptError::headless_no_answer(message, key);
        return Err(ArchetypeScriptErrorWrapper(call, error).into());
    }


    runtime_context.request(CommandRequest::PromptForText(prompt_info.clone()));

    match runtime_context.response() {
        CommandResponse::String(answer) => {
            match validate_text(min, max, &answer.to_string()) {
                Ok(_) => Ok(answer.into()),
                Err(error_message) => {
                    let error = ArchetypeScriptError::answer_validation_error(answer.to_string(), message, key, error_message);
                    return Err(ArchetypeScriptErrorWrapper(call, error).into());
                }
            }
        }
        CommandResponse::None => {
            if !prompt_info.optional() {
                let error = ArchetypeScriptError::answer_not_optional(message, key);
                return Err(ArchetypeScriptErrorWrapper(call, error).into());
            } else {
                return Ok(None);
            }
        }
        CommandResponse::Error(error) => {
            return Err(ArchetypeScriptErrorWrapper(call, ArchetypeScriptError::PromptError(error)).into());
        }
        response => {
            let error = ArchetypeScriptError::unexpected_prompt_response(message, key, "a String", response);
            return Err(ArchetypeScriptErrorWrapper(call, error).into());
        }
    }
}
