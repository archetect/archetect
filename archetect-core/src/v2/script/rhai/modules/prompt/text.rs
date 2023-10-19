use crate::errors::ArchetectError;
use crate::v2::runtime::context::RuntimeContext;
use crate::v2::script::rhai::modules::prompt::handle_result;
use inquire::validator::Validation;
use inquire::Text;
use rhai::{Dynamic, EvalAltResult, Map, NativeCallContext};
use std::ops::{RangeFrom, RangeInclusive, RangeToInclusive};

pub fn prompt(
    call: NativeCallContext,
    message: &str,
    settings: &Map,
    runtime_context: &RuntimeContext,
    key: Option<&str>,
    answer: Option<&Dynamic>,
) -> Result<String, Box<EvalAltResult>> {
    let min = settings
        .get("min")
        .map(|value| value.to_string().parse::<i64>())
        .map(|value| value.ok())
        .flatten()
        .or(Some(1));

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
        return match validate(min, max, &answer.to_string()) {
            Ok(_) => Ok(answer.to_string()),
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
    }

    let mut text = Text::new(message).with_validator(validator);

    let _optional = settings
        .get("optional")
        .map_or(Ok(false), |value| value.as_bool())
        .unwrap_or(false);

    if let Some(default_value) = settings.get("defaults_with") {
        if runtime_context.headless() {
            return Ok(default_value.to_string());
        } else {
            text.default = Some(default_value.to_string());
        }
    }

    if runtime_context.headless() {
        let fn_name = call.fn_name().to_owned();
        let source = call.source().unwrap_or_default().to_owned();
        let position = call.position();
        let error = EvalAltResult::ErrorSystem(
            "Headless Mode Error".to_owned(),
            Box::new(ArchetectError::GeneralError(if let Some(key) = key {
                format!("{} for '{}'", message, key,).to_owned()
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
        text.placeholder = Some(placeholder.to_string());
    }

    if let Some(help_message) = settings.get("help") {
        text.help_message = Some(help_message.to_string());
    }

    let result = text.prompt();

    handle_result(result)
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
