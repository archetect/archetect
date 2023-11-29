use std::ops::{RangeFrom, RangeInclusive, RangeToInclusive};

use rhai::{Dynamic, EvalAltResult, Map, NativeCallContext};

use archetect_api::{CommandRequest, CommandResponse, TextPromptInfo};

use crate::errors::ArchetectError;
use crate::runtime::context::RuntimeContext;
use crate::script::rhai::modules::prompt::{get_optional_setting, parse_setting};

pub fn prompt<K: AsRef<str>>(
    call: NativeCallContext,
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
        return match validate(min, max, &answer.to_string()) {
            Ok(_) => Ok(answer.clone()),
            Err(message) => {
                let fn_name = call.fn_name().to_owned();
                let source = call.source().unwrap_or_default().to_owned();
                let position = call.position();
                let error = EvalAltResult::ErrorSystem(
                    "Invalid Answer".to_owned(),
                    Box::new(ArchetectError::GeneralError(if let Some(key) = key {
                        format!("{} for '{}'", message, key.as_ref(),).to_owned()
                    } else {
                        format!("{}", message).to_owned()
                    })),
                );
                Err(Box::new(EvalAltResult::ErrorInFunctionCall(
                    fn_name,
                    source,
                    Box::new(error),
                    position,
                )))
            }
        };
    }


    let mut prompt_info = TextPromptInfo::new(message);

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

        let fn_name = call.fn_name().to_owned();
        let source = call.source().unwrap_or_default().to_owned();
        let position = call.position();
        let error = EvalAltResult::ErrorSystem(
            "Headless Mode Error".to_owned(),
            Box::new(ArchetectError::GeneralError(if let Some(key) = key {
                format!("{} for '{}'", message, key.as_ref(),).to_owned()
            } else {
                format!("{}", message).to_owned()
            })),
        );
        return Err(Box::new(EvalAltResult::ErrorInFunctionCall(
            fn_name,
            source,
            Box::new(error),
            position,
        )));
    }

    if let Some(placeholder) = settings.get("placeholder") {
        prompt_info = prompt_info.with_placeholder(Some(placeholder.to_string()));
    }

    if let Some(help_message) = settings.get("help") {
        prompt_info = prompt_info.with_placeholder(Some(help_message.to_string()));
    } else {
        if optional {
            prompt_info = prompt_info.with_placeholder(Some("<esc> for None"));
        }
    }

    runtime_context.request(CommandRequest::PromptForText(prompt_info));

    match runtime_context.responses().lock().unwrap().recv().expect("Error Receiving Response") {
        CommandResponse::StringAnswer(answer) => {
            return Ok(answer.into());
        }
        CommandResponse::NoneAnswer => {
            return Ok(Dynamic::UNIT);
        }
        CommandResponse::Error(error) => {
            let error = EvalAltResult::ErrorSystem("Prompt Error".to_string(), Box::new(ArchetectError::NakedError(error)));
            return Err(Box::new(error));
        }
        response => {
            let error = EvalAltResult::ErrorSystem("Invalid Answer Type".to_string(), Box::new(ArchetectError::NakedError(format!("{:?}", response))));
            return Err(Box::new(error));
        }
    }
}

fn validate(min: Option<i64>, max: Option<i64>, input: &str) -> Result<(), String> {
    let length = input.len() as i64;
    match (min, max) {
        (Some(start), Some(end)) => {
            if !RangeInclusive::new(start, end).contains(&length) {
                return Err(format!("Answer must be between {} and {}", start, end));
            }
        }
        (Some(start), None) => {
            if !(RangeFrom { start }.contains(&length)) {
                return Err(format!("Answer must be greater than {}", start));
            }
        }
        (None, Some(end)) => {
            if !(RangeToInclusive { end }.contains(&length)) {
                return Err(format!("Answer must be less than or equal to {}", end));
            }
        }
        (None, None) => return Ok(()),
    };

    Ok(())
}
