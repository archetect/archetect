use log::warn;
use rhai::{Dynamic, EvalAltResult, Map, NativeCallContext};

use inquire::{InquireError, MultiSelect};

use crate::v2::runtime::context::RuntimeContext;
use crate::v2::script::rhai::modules::prompt::create_error_from_call;
use crate::{ArchetectError, ArchetypeError};

pub fn prompt(
    call: NativeCallContext,
    message: &str,
    options: Vec<Dynamic>,
    runtime_context: &RuntimeContext,
    settings: &Map,
    key: Option<&str>,
    answer: Option<&Dynamic>,
) -> Result<Vec<Dynamic>, Box<EvalAltResult>> {
    let mut prompt = MultiSelect::new(message, options.clone());
    let mut indices = vec![];

    if let Some(defaults_with) = settings.get("defaults_with") {
        if let Some(defaults) = defaults_with.clone().try_cast::<Vec<Dynamic>>() {
            for default in defaults.iter() {
                if let Some(position) = options
                    .iter()
                    .position(|option| option.to_string().as_str() == default.to_string().as_str())
                {
                    indices.push(position);
                }
            }

            if runtime_context.headless() {
                let mut results = vec![];
                for index in indices {
                    results.push(options.get(index).unwrap().clone_cast::<Dynamic>());
                }
                return Ok(results);
            } else {
                prompt.default = Some(indices.as_slice());
            }
        } else {
            let error = create_error_from_call(
                &call,
                "Invalid Default Type",
                ArchetectError::GeneralError(if let Some(key) = key {
                    format!(
                        "'{}' ({}) was provided as a default for '{}', but must be an array of values.",
                        defaults_with,
                        defaults_with.type_name(),
                        key
                    )
                    .to_owned()
                } else {
                    message.to_string()
                }),
            );
            return Err(error);
        }
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
        Ok(selections) => Ok(selections),
        Err(err) => match err {
            InquireError::OperationCanceled => Err(Box::new(EvalAltResult::ErrorSystem(
                "Cancelled".to_owned(),
                Box::new(ArchetypeError::ValueRequired),
            ))),
            InquireError::OperationInterrupted => Err(Box::new(EvalAltResult::ErrorSystem(
                "Cancelled".to_owned(),
                Box::new(ArchetypeError::OperationInterrupted),
            ))),
            err => Err(Box::new(EvalAltResult::ErrorSystem("Error".to_owned(), Box::new(err)))),
        },
    }
}
