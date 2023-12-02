use std::borrow::Cow;

use rhai::{Dynamic, EvalAltResult, Map, NativeCallContext};

use archetect_api::{BoolPromptInfo, CommandRequest, CommandResponse, PromptInfo};

use crate::errors::{ArchetectError, ArchetypeError};
use crate::runtime::context::RuntimeContext;
use crate::script::rhai::modules::prompt::get_optional_setting;
use crate::utils::{ArchetypeRhaiFunctionError, ArchetypeRhaiSystemError};

// TODO: Better help messages
pub fn prompt<'a, K: Into<Cow<'a, str>>>(
    call: &NativeCallContext,
    message: &str,
    runtime_context: &RuntimeContext,
    settings: &Map,
    key: Option<K>,
    answer: Option<&Dynamic>,
) -> Result<Dynamic, Box<EvalAltResult>> {
    if let Some(answer) = answer {
        return match get_boolean(answer.to_string().as_str()) {
            Ok(value) => Ok(value.into()),
            Err(_) => {
                let error = ArchetypeError::answer_validation_error(answer.to_string(), message, key, "must resemble a boolean");
                Err(ArchetypeRhaiFunctionError("Invalid Answer", call, error).into())
            }
        }
    }

    let optional = get_optional_setting(settings);

    let mut prompt_info = BoolPromptInfo::new(message);

    if let Some(default_value) = settings.get("defaults_with") {
        match get_boolean(default_value.to_string().as_str()) {
            Ok(default) => {
                if runtime_context.headless() {
                    return Ok(default.into());
                } else {
                    prompt_info = prompt_info.with_default(Some(default));
                }
            }
            Err(_) => {
                let error = ArchetypeError::answer_validation_error(default_value.to_string(), message, key, "must resemble a boolean");
                return Err(ArchetypeRhaiFunctionError("Invalid Default", call, error).into());
            }
        }
    }

    if runtime_context.headless() {
        let error = ArchetypeError::headless_no_answer(message, key);
        return Err(ArchetypeRhaiFunctionError("Headless", call, error).into());
    }

    if let Some(placeholder) = settings.get("placeholder") {
        prompt_info = prompt_info.with_placeholder(Some(placeholder.to_string()));
    }

    if let Some(help_message) = settings.get("help") {
        prompt_info = prompt_info.with_help(Some(help_message.to_string()));
    } else {
        if optional {
            prompt_info = prompt_info.with_help(Some("<esc> for None".to_string()));
        }
    }

    runtime_context.request(CommandRequest::PromptForBool(prompt_info.clone()));

    match runtime_context.response() {
        CommandResponse::Boolean(answer) => {
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
            let error =
                EvalAltResult::ErrorSystem("Prompt Error".to_string(), Box::new(ArchetectError::NakedError(error)));
            return Err(Box::new(error));
        }
        response => {
            let error = EvalAltResult::ErrorSystem(
                "Invalid Answer Type".to_string(),
                Box::new(ArchetectError::NakedError(format!("{:?}", response))),
            );
            return Err(Box::new(error));
        }
    }
}

fn get_boolean<V: AsRef<str>>(value: V) -> Result<bool, ()> {
    match value.as_ref().to_lowercase().as_str() {
        "y" | "yes" | "t" | "true" => Ok(true),
        "n" | "no" | "f" | "false" => Ok(false),
        _ => Err(()),
    }
}
