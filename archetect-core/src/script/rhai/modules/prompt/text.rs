use std::borrow::Cow;

use rhai::{Dynamic, EvalAltResult, Map, NativeCallContext};

use archetect_api::validations::validate_text;
use archetect_api::{CommandRequest, CommandResponse, PromptInfo, TextPromptInfo};

use crate::errors::{ArchetypeScriptError, ArchetypeScriptErrorWrapper};
use crate::runtime::context::RuntimeContext;
use crate::script::rhai::modules::prompt::{get_optional_setting, parse_setting};

pub fn prompt<'a, K: Into<Cow<'a, str>>>(
    call: &NativeCallContext,
    message: &str,
    settings: &Map,
    runtime_context: &RuntimeContext,
    key: Option<K>,
    answer: Option<&Dynamic>,
) -> Result<Dynamic, Box<EvalAltResult>> {
    let optional = get_optional_setting(settings);
    let min = parse_setting("min", settings).or(Some(1));
    let max = parse_setting("max", settings);

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

    let mut prompt_info = TextPromptInfo::new(message)
        .with_min(min)
        .with_max(max)
        .with_optional(optional);

    if let Some(default_value) = settings.get("defaults_with") {
        if runtime_context.headless() {
            return Ok(default_value.clone());
        } else {
            prompt_info = prompt_info.with_default(Some(default_value.to_string()));
        }
    }

    if runtime_context.headless() {
        // If we're headless, and there was no default, but this is an optional prompt,
        // return a UNIT
        if optional {
            return Ok(Dynamic::UNIT);
        }
        let error = ArchetypeScriptError::headless_no_answer(message, key);
        return Err(ArchetypeScriptErrorWrapper(call, error).into());
    }

    if let Some(placeholder) = settings.get("placeholder") {
        prompt_info = prompt_info.with_placeholder(Some(placeholder.to_string()));
    }

    if let Some(help_message) = settings.get("help") {
        prompt_info = prompt_info.with_help(Some(help_message.to_string()));
    } else {
        if optional {
            prompt_info = prompt_info.with_help(Some("<esc> for None"));
        }
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
                return Ok(Dynamic::UNIT);
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
