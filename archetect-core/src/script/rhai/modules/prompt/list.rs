use std::borrow::Cow;
use rhai::{Dynamic, EvalAltResult, Map, NativeCallContext};

use archetect_api::{CommandRequest, CommandResponse, ListPromptInfo, PromptInfo};

use crate::errors::{ArchetectError, ArchetypeError};
use crate::runtime::context::RuntimeContext;
use crate::script::rhai::modules::prompt::{get_optional_setting, parse_setting};
use crate::utils::{ArchetypeRhaiFunctionError, ArchetypeRhaiSystemError};

pub fn prompt<'a, K: Into<Cow<'a, str>>>(
    call: &NativeCallContext,
    message: &str,
    runtime_context: &RuntimeContext,
    settings: &Map,
    key: Option<K>,
    answer: Option<&Dynamic>,
) -> Result<Dynamic, Box<EvalAltResult>> {
    if let Some(answer) = answer {
        if let Some(answer) = answer.clone().try_cast::<String>() {
            let answers = answer
                .split(',')
                .map(|v| v.trim())
                .map(|v| v.to_owned())
                .collect::<Vec<String>>();
            return Ok(answers.into());
        }

        if let Some(answers) = answer.clone().try_cast::<Vec<Dynamic>>() {
            let answers = answers.iter().map(|v| v.to_string()).collect::<Vec<String>>();
            return Ok(answers.into());
        }

        let requirement = " must be an array of values or a comma-separated string".to_string();
        let error = if let Some(key) = key {
            ArchetypeError::KeyedAnswerValidationError {
                answer: answer.to_string(),
                prompt: message.to_string(),
                key: key.into().to_string(),
                requires: requirement,
            }
        } else {
            ArchetypeError::AnswerValidationError {
                answer: answer.to_string(),
                prompt: message.to_string(),
                requires: requirement,
            }
        };
        return Err(ArchetypeRhaiFunctionError("Invalid Answer", call, error).into());
    }

    let mut prompt_info = ListPromptInfo::new(message)
        .with_optional(get_optional_setting(settings))
        .with_min_items(parse_setting::<usize>("min_items", settings))
        .with_max_items(parse_setting::<usize>("max_items", settings));

    if let Some(default_value) = settings.get("defaults_with") {
        if let Some(defaults) = default_value.clone().try_cast::<Vec<String>>() {
            if runtime_context.headless() {
                return Ok(defaults.into());
            } else {
                prompt_info = prompt_info.with_defaults(Some(defaults));
            }
        } else {
            // TODO: Throw error about wrong type
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
        prompt_info = prompt_info.with_placeholder(Some(help_message.to_string()));
    }

    runtime_context.request(CommandRequest::PromptForList(prompt_info.clone()));

    match runtime_context.response() {
        CommandResponse::Array(answer) => {
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
