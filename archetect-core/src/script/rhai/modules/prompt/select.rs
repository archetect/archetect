use crate::errors::{ArchetectError, ArchetypeError};
use crate::runtime::context::RuntimeContext;
use inquire::{InquireError, Select};
use log::warn;
use rhai::{Dynamic, EvalAltResult, Map, NativeCallContext};

pub fn prompt(
    call: NativeCallContext,
    message: &str,
    options: Vec<Dynamic>,
    runtime_context: &RuntimeContext,
    settings: &Map,
    key: Option<&str>,
    answer: Option<&Dynamic>,
) -> Result<String, Box<EvalAltResult>> {
    let options = &options;

    if let Some(answer) = answer {
        for option in options {
            if option.to_string().as_str().to_lowercase() == answer.to_string().as_str().to_lowercase() {
                return Ok(option.to_string());
            }
        }

        let fn_name = call.fn_name().to_owned();
        let source = call.source().unwrap_or_default().to_owned();
        let position = call.position();
        let error = EvalAltResult::ErrorSystem(
            "Invalid Answer".to_owned(),
            Box::new(ArchetectError::GeneralError(if let Some(key) = key {
                format!(
                    "'{}' was provided as an answer to '{}', but did not match any of the required options.",
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
    };

    let default = if let Some(defaults_with) = settings.get("defaults_with") {
        let default = options
            .iter()
            .position(|item| item.to_string().as_str() == defaults_with.to_string().as_str());
        if default.is_none() {
            warn!("A 'defaults_with' was set, but did not match any of the options.")
        }
        default
    } else {
        None
    };

    let mut prompt = Select::new(message, options.to_vec());

    if let Some(default) = default {
        if runtime_context.headless() {
            return Ok(options.get(default).unwrap().to_string());
        }
        prompt.starting_cursor = default;
    }

    if runtime_context.headless() {
        return Err(Box::new(EvalAltResult::ErrorSystem(
            "Headless Mode Error".to_owned(),
            Box::new(ArchetectError::HeadlessNoDefault),
        )));
    }

    if let Some(page_size) = settings.get("page_size") {
        if let Some(page_size) = page_size.clone().try_cast::<i64>() {
            prompt.page_size = page_size as usize;
        } else {
            warn!(
                "Invalid data type used for 'page_size': {}; should be an integer",
                page_size.type_name()
            );
        }
    } else {
        prompt.page_size = 10;
    }

    if let Some(help_message) = settings.get("help") {
        prompt.help_message = Some(help_message.to_string());
    }

    let result = prompt.prompt();

    match result {
        Ok(selection) => Ok(selection.to_string()),
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
            err => Err(Box::new(EvalAltResult::ErrorSystem("Error".to_owned(), Box::new(err)))),
        },
    }
}
