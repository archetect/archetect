use crate::errors::ArchetectError;
use crate::v2::runtime::context::RuntimeContext;
use crate::v2::script::rhai::modules::prompt::handle_result;
use inquire::validator::Validation;
use inquire::List;
use rhai::{Dynamic, EvalAltResult, Map, NativeCallContext};
use std::ops::{RangeFrom, RangeInclusive, RangeToInclusive};

pub fn prompt(
    call: NativeCallContext,
    message: &str,
    settings: &Map,
    runtime_context: &RuntimeContext,
    key: Option<&str>,
    answer: Option<&Dynamic>,
) -> Result<Vec<String>, Box<EvalAltResult>> {
    let min_items = settings
        .get("min_items")
        .map(|value| value.to_string().parse::<usize>())
        .map(|value| value.ok())
        .flatten();


    let max_items = settings
        .get("max_items")
        .map(|value| value.to_string().parse::<usize>())
        .map(|value| value.ok())
        .flatten();

    let list_validator = move |input: &Vec<String>| match validate_list(min_items, max_items, input) {
        Ok(_) => return Ok(Validation::Valid),
        Err(message) => return Ok(Validation::Invalid(message.into())),
    };

    if let Some(answer) = answer {
        if let Some(answer) = answer.clone().try_cast::<String>() {
            let answers = answer
                .split(',')
                .map(|v| v.trim())
                .map(|v| v.to_owned())
                .collect::<Vec<String>>();
            return Ok(answers);
        }

        if let Some(answers) = answer.clone().try_cast::<Vec<Dynamic>>() {
            let answers = answers.iter().map(|v| v.to_string()).collect::<Vec<String>>();
            return Ok(answers);
        }

        let fn_name = call.fn_name().to_owned();
        let source = call.source().unwrap_or_default().to_owned();
        let position = call.position();
        let error = EvalAltResult::ErrorSystem(
            "Invalid Answer Type".to_owned(),
            Box::new(ArchetectError::GeneralError(if let Some(key) = key {
                format!(
                    "'{}' was provided as an answer to '{}', but must be an array of values or a comma-separated string.",
                    answer, key
                )
                    .to_owned()
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

    let mut list = List::new(message).with_list_validator(list_validator);

    let _optional = settings
        .get("optional")
        .map_or(Ok(false), |value| value.as_bool())
        .unwrap_or(false);

    if let Some(default_value) = settings.get("defaults_with") {
        if let Some(defaults) = default_value.clone().try_cast::<Vec<Dynamic>>() {
            let defaults = defaults
                .iter()
                .map(|i| i.to_string())
                .collect::<Vec<String>>();
            if runtime_context.headless() {
                return Ok(defaults);
            } else {
                list.default = Some(defaults);
            }
        } else {

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
        list.placeholder = Some(placeholder.to_string());
    }

    if let Some(help_message) = settings.get("help") {
        list.help_message = Some(help_message.to_string());
    }

    let result = list.prompt();

    handle_result(result)
}

fn validate_list(min_items: Option<usize>, max_items: Option<usize>, input: &Vec<String>) -> Result<(), String> {
    let length = input.len();
    match (min_items, max_items) {
        (Some(start), Some(end)) => {
            if !RangeInclusive::new(start, end).contains(&input.len()) {
                return Err(format!("List must have between {} and {} items", start, end));
            }
        }
        (Some(start), None) => {
            if !(RangeFrom { start }.contains(&length)) {
                return Err(format!("List must have at least {} items", start));
            }
        }
        (None, Some(end)) => {
            if !(RangeToInclusive { end }.contains(&length)) {
                return Err(format!("List must have no more than {} items", end));
            }
        }
        (None, None) => return Ok(()),
    };

    Ok(())
}
