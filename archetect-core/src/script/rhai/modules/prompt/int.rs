use log::warn;
use rhai::{Dynamic, EvalAltResult, Map, NativeCallContext};

use archetect_api::validations::validate_int;
use archetect_api::{CommandRequest, CommandResponse, IntPromptInfo, PromptInfo};

use crate::errors::{ArchetypeScriptError, ArchetypeScriptErrorWrapper};
use crate::runtime::context::RuntimeContext;
use crate::script::rhai::modules::prompt::{cast_setting, parse_setting};

pub fn prompt_int<'a, K: AsRef<str> + Clone>(
    call: &NativeCallContext,
    message: &str,
    runtime_context: &RuntimeContext,
    settings: &Map,
    key: Option<K>,
    answer: Option<&Dynamic>,
) -> Result<Dynamic, Box<EvalAltResult>> {
    let optional = cast_setting("optional", settings, message, key.clone())
        .map_err(|error| ArchetypeScriptErrorWrapper(call, error))?
        .unwrap_or_default();
    let min = parse_setting("min", settings, message, key.clone())
        .map_err(|error| ArchetypeScriptErrorWrapper(call, error))?;
    let max = parse_setting("max", settings, message, key.clone())
        .map_err(|error| ArchetypeScriptErrorWrapper(call, error))?;
    let placeholder = cast_setting("placeholder", settings, message, key.clone())
        .map_err(|error| ArchetypeScriptErrorWrapper(call, error))?;
    let help = cast_setting("help", settings, message, key.clone())
        .map_err(|error| ArchetypeScriptErrorWrapper(call, error))?
        .or_else(|| if optional { Some("<esc> for None".to_string()) } else { None })
        ;

    let mut prompt_info = IntPromptInfo::new(message)
        .with_optional(optional)
        .with_min(min)
        .with_max(max)
        .with_placeholder(placeholder)
        .with_help(help);

    if let Some(answer) = answer {
        return if let Some(answer) = answer.clone().try_cast::<i64>() {
            match validate_int(min, max, answer) {
                Ok(_) => Ok(answer.into()),
                Err(error_message) => {
                    let error =
                        ArchetypeScriptError::answer_validation_error(answer.to_string(), message, key, error_message);
                    return Err(ArchetypeScriptErrorWrapper(call, error).into());
                }
            }
        } else {
            let error = ArchetypeScriptError::answer_type_error(answer.to_string(), message, key, "an Integer");
            Err(ArchetypeScriptErrorWrapper(call, error).into())
        };
    }

    if let Some(default_value) = settings.get("defaults_with") {
        let default_value = default_value.to_string();
        match default_value.parse::<i64>() {
            Ok(value) => {
                if runtime_context.headless() {
                    return Ok(value.into());
                } else {
                    prompt_info = prompt_info.with_default(Some(value));
                }
            }
            // TODO: return error
            Err(_) => warn!("Default for prompt should be an 'int'', but was '{})", default_value),
        }
    }

    if runtime_context.headless() {
        let error = ArchetypeScriptError::headless_no_answer(message, key);
        return Err(ArchetypeScriptErrorWrapper(call, error).into());
    }

    runtime_context.request(CommandRequest::PromptForInt(prompt_info.clone()));

    match runtime_context.response() {
        CommandResponse::Integer(answer) => match validate_int(min, max, answer) {
            Ok(_) => Ok(answer.into()),
            Err(error_message) => {
                let error =
                    ArchetypeScriptError::answer_validation_error(answer.to_string(), message, key, error_message);
                return Err(ArchetypeScriptErrorWrapper(call, error).into());
            }
        },
        CommandResponse::None => {
            if !prompt_info.optional() {
                let error = ArchetypeScriptError::answer_not_optional(message, key);
                return Err(ArchetypeScriptErrorWrapper(call, error).into());
            } else {
                return Ok(Dynamic::UNIT);
            }
        }
        CommandResponse::Error(error) => {
            return Err(ArchetypeScriptErrorWrapper(call, ArchetypeScriptError::PromptError(error)).into());
        }
        response => {
            let error = ArchetypeScriptError::unexpected_prompt_response(message, key, "Int", response);
            return Err(ArchetypeScriptErrorWrapper(call, error).into());
        }
    }
}
