use crate::errors::{ArchetectError, ArchetypeError};
use crate::v2::runtime::context::RuntimeContext;
use inquire::validator::Validation;
use inquire::{InquireError, Text};
use log::warn;
use rhai::{Dynamic, EvalAltResult, Map, NativeCallContext};
use std::ops::{RangeFrom, RangeInclusive, RangeToInclusive};

pub fn prompt(
    call: NativeCallContext,
    message: &str,
    runtime_context: &RuntimeContext,
    settings: &Map,
    key: Option<&str>,
    answer: Option<&Dynamic>,
) -> Result<i64, Box<EvalAltResult>> {
    let mut text = Text::new(message);

    let min = settings
        .get("min")
        .map(|value| value.to_string().parse::<i64>())
        .map(|value| value.ok())
        .flatten();

    let max = settings
        .get("max")
        .map(|value| value.to_string().parse::<i64>())
        .map(|value| value.ok())
        .flatten();

    let validator = move |input: &str| match validate(min, max, input) {
        Ok(_) => return Ok(Validation::Valid),
        Err(message) => return Ok(Validation::Invalid(message.into())),
    };

    if let Some(answer) = answer {
        if let Some(answer) = answer.clone().try_cast::<i64>() {
            return match validate(min, max, &answer.to_string()) {
                Ok(_) => Ok(answer),
                Err(message) => {
                    let fn_name = call.fn_name().to_owned();
                    let source = call.source().unwrap_or_default().to_owned();
                    let position = call.position();
                    let error = EvalAltResult::ErrorSystem(
                        "Invalid Answer".to_owned(),
                        Box::new(ArchetectError::GeneralError(if let Some(key) = key {
                            format!("{} for '{}'", message, key,).to_owned()
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
                        key,
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

    let _optional = settings
        .get("optional")
        .map_or(Ok(false), |value| value.as_bool())
        .unwrap_or(false);

    if let Some(default_value) = settings.get("defaults_with") {
        let default_value = default_value.to_string();
        match default_value.parse::<i64>() {
            Ok(value) => {
                if runtime_context.headless() {
                    return Ok(value);
                } else {
                    text.default = Some(default_value.to_string());
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
        text.placeholder = Some(placeholder.to_string());
    }

    if let Some(help_message) = settings.get("help") {
        text.help_message = Some(help_message.to_string());
    }

    text = text.with_validator(validator);

    let result = text.prompt();

    match result {
        Ok(value) => return Ok(value.parse::<i64>().unwrap()),
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
            err => return Err(Box::new(EvalAltResult::ErrorSystem("Error".to_owned(), Box::new(err)))),
        },
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
}
