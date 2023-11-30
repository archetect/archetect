use std::ops::{RangeFrom, RangeInclusive, RangeToInclusive};

use log::warn;
use rhai::{Dynamic, EvalAltResult, Map, NativeCallContext};

use archetect_api::{CommandRequest, CommandResponse, IntPromptInfo};

use crate::errors::ArchetectError;
use crate::runtime::context::RuntimeContext;
use crate::script::rhai::modules::prompt::{get_optional_setting, parse_setting};

pub fn prompt<K: AsRef<str>>(
    call: NativeCallContext,
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
        .with_optional(optional)
        ;

    if let Some(answer) = answer {
        if let Some(answer) = answer.clone().try_cast::<i64>() {
            return match validate(min, max, &answer.to_string()) {
                Ok(_) => Ok(answer.into()),
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
        } else {
            let fn_name = call.fn_name().to_owned();
            let source = call.source().unwrap_or_default().to_owned();
            let position = call.position();
            let error = EvalAltResult::ErrorSystem(
                "Invalid Answer".to_owned(),
                Box::new(ArchetectError::GeneralError(if let Some(key) = key {
                    format!(
                        "'{}' expects an answer of type 'int', but was answered with '{}', which is of type '{}'",
                        key.as_ref(),
                        answer,
                        answer.type_name(),
                    )
                    .to_owned()
                } else {
                    format!("Expected answer as an 'int'', but was of type '{}'", answer.type_name()).to_owned()
                })),
            );
            return Err(Box::new(EvalAltResult::ErrorInFunctionCall(
                fn_name,
                source,
                Box::new(error),
                position,
            )));
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
        return Err(Box::new(EvalAltResult::ErrorSystem(
            "Headless Mode Error".to_owned(),
            Box::new(ArchetectError::HeadlessNoDefault),
        )));
    }

    if let Some(placeholder) = settings.get("placeholder") {
        prompt_info = prompt_info.with_placeholder(Some(placeholder.to_string()));
    }

    if let Some(help_message) = settings.get("help") {
        prompt_info = prompt_info.with_help(Some(help_message.to_string()));
    }

    runtime_context.request(CommandRequest::PromptForInt(prompt_info));

    match runtime_context.response() {
        CommandResponse::IntAnswer(answer) => {
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

