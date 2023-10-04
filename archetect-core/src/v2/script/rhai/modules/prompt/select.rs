use crate::v2::runtime::context::RuntimeContext;
use crate::{ArchetectError, ArchetypeError};
use inquire::{InquireError, Select};
use log::warn;
use rhai::{Dynamic, EvalAltResult, Map, NativeCallContext};

// TODO: Implement Defaults
// TODO: Handle Answers
pub fn prompt(
    _call: NativeCallContext,
    message: &str,
    options: Vec<Dynamic>,
    runtime_context: &RuntimeContext,
    settings: &Map,
) -> Result<String, Box<EvalAltResult>> {
    let options = &options;
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