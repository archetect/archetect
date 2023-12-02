use std::borrow::Cow;

use rhai::{Dynamic, EvalAltResult, Map, NativeCallContext};

use archetect_api::{CommandRequest, CommandResponse, PromptInfo, TextPromptInfo};
use archetect_api::validations::validate_text;

use crate::errors::{ArchetectError, ArchetypeError};
use crate::runtime::context::RuntimeContext;
use crate::script::rhai::modules::prompt::{get_optional_setting, parse_setting};
use crate::utils::{ArchetectRhaiSystemError, ArchetypeRhaiFunctionError, ArchetypeRhaiSystemError};

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
        return match validate_text(min, max, &answer.to_string()) {
            Ok(_) => Ok(answer.clone()),
            Err(error_message) => {
                let error = ArchetypeError::answer_validation_error(answer.to_string(), message, key, error_message);
                Err(ArchetypeRhaiFunctionError("Invalid Answer", call, error).into())
            }
        };
    }

    let mut prompt_info = TextPromptInfo::new(message)
        .with_min(min)
        .with_max(max)
        .with_optional(optional)
        ;

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
        let error = ArchetypeError::headless_no_answer(message, key);
        return Err(ArchetypeRhaiFunctionError("Headless Mode", call, error).into());
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
            // TODO: Validate response from Driver
            return Ok(answer.into());
        }
        CommandResponse::None => {
            if !prompt_info.optional() {
                let error = ArchetypeError::answer_not_optional(message, key);
                return Err(ArchetypeRhaiSystemError("Required", error).into());
            } else {
                return Ok(Dynamic::UNIT);
            }
        }
        CommandResponse::Error(error) => {
            let error = ArchetectError::NakedError(error);
            return Err(ArchetectRhaiSystemError("Prompt Error", error).into());
        }
        response => {
            let error = ArchetectError::NakedError(format!(
                "'{}' requires a String, but was answered with {:?}",
                prompt_info.message(),
                response
            ));
            return Err(ArchetectRhaiSystemError("Invalid Type", error).into());
        }
    }
}
