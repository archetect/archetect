use std::borrow::Cow;
use std::ops::{RangeFrom, RangeInclusive, RangeToInclusive};

use log::warn;
use rhai::{Dynamic, EvalAltResult, Map, NativeCallContext};

use archetect_api::{CommandRequest, CommandResponse, IntPromptInfo, PromptInfo};

use crate::errors::{ArchetypeScriptError, ArchetypeScriptErrorWrapper};
use crate::runtime::context::RuntimeContext;
use crate::script::rhai::modules::prompt::{get_optional_setting, parse_setting};

pub fn prompt<'a, K: Into<Cow<'a, str>>>(
    call: &NativeCallContext,
    message: &str,
    runtime_context: &RuntimeContext,
    settings: &Map,
    key: Option<K>,
    answer: Option<&Dynamic>,
) -> Result<Dynamic, Box<EvalAltResult>> {
    let optional = get_optional_setting(settings);
    let min = parse_setting::<i64>("min", settings);
    let max = parse_setting::<i64>("max", settings);

    let mut prompt_info = IntPromptInfo::new(message)
        .with_min(min)
        .with_max(max)
        .with_optional(optional);

    if let Some(answer) = answer {
        if let Some(answer) = answer.clone().try_cast::<i64>() {
            return match validate(min, max, &answer.to_string()) {
                Ok(_) => Ok(answer.into()),
                Err(error_message) => {
                    let error = ArchetypeScriptError::answer_validation_error(answer.to_string(), message, key, error_message);
                    return Err(ArchetypeScriptErrorWrapper(call, error).into());
                }
            };
        } else {
            let error = ArchetypeScriptError::answer_type_error(answer.to_string(), message, key, "an Int");
            return Err(ArchetypeScriptErrorWrapper(call, error).into());
        }
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

    if let Some(placeholder) = settings.get("placeholder") {
        prompt_info = prompt_info.with_placeholder(Some(placeholder.to_string()));
    }

    if let Some(help_message) = settings.get("help") {
        prompt_info = prompt_info.with_help(Some(help_message.to_string()));
    }

    runtime_context.request(CommandRequest::PromptForInt(prompt_info.clone()));

    match runtime_context.response() {
        CommandResponse::Integer(answer) => {
            return Ok(answer.into());
        }
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

fn validate(min: Option<i64>, max: Option<i64>, input: &str) -> Result<(), String> {
    match input.parse::<i64>() {
        Ok(value) => {
            match (min, max) {
                (Some(start), Some(end)) => {
                    if !RangeInclusive::new(start, end).contains(&value) {
                        return Err(format!("Answer must be between {} and {}", start, end));
                    }
                }
                (Some(start), None) => {
                    if !(RangeFrom { start }.contains(&value)) {
                        return Err(format!("Answer must be greater than {}", start));
                    }
                }
                (None, Some(end)) => {
                    if !(RangeToInclusive { end }.contains(&value)) {
                        return Err(format!("Answer must be less than or equal to {}", end));
                    }
                }
                (None, None) => {}
            };

            Ok(())
        }
        Err(_) => Err(format!("{} is not an 'int'", input)),
    }
}
