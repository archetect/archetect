use log::warn;
use rhai::{Dynamic, EvalAltResult, Map, NativeCallContext};

use inquire::{MultiSelect};

use crate::errors::{ArchetectError};
use crate::runtime::context::RuntimeContext;
use crate::script::rhai::modules::prompt::{create_error_from_call, get_optional_setting, get_render_config, handle_result};

pub fn prompt<K: AsRef<str>>(
    call: NativeCallContext,
    message: &str,
    options: Vec<Dynamic>,
    runtime_context: &RuntimeContext,
    settings: &Map,
    key: Option<K>,
    answer: Option<&Dynamic>,
) -> Result<Dynamic, Box<EvalAltResult>> {
    let optional = get_optional_setting(settings);

    let mut prompt = MultiSelect::new(message, options.clone())
        .with_render_config(get_render_config())
        ;

    let mut indices = vec![];

    if let Some(answer) = answer {
        // Handle an answer as a comma-separated string
        if let Some(answer) = answer.clone().try_cast::<String>() {
            let mut results = vec![];
            let answers = answer.split(',').map(|v| v.trim()).collect::<Vec<&str>>();
            for answer in answers {
                if let Some(result) = options
                    .iter()
                    .find(|option| option.to_string().as_str().to_lowercase() == answer.to_lowercase())
                {
                    results.push(result.clone())
                } else {
                    let fn_name = call.fn_name().to_owned();
                    let source = call.source().unwrap_or_default().to_owned();
                    let position = call.position();
                    let error = EvalAltResult::ErrorSystem(
                        "Invalid Answer".to_owned(),
                        Box::new(ArchetectError::GeneralError(if let Some(key) = key {
                            format!(
                                "'{}' was provided as an answer to '{}', but did not match any of the required options.",
                                answer, key.as_ref()
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
            }

            return Ok(results.into());
        }
        // Handle an answer as an array of values
        if let Some(answers) = answer.clone().try_cast::<Vec<Dynamic>>() {
            let mut results = vec![];
            for answer in answers {
                if let Some(result) = options.iter().find(|option| {
                    option.to_string().as_str().to_lowercase() == answer.to_string().as_str().to_lowercase()
                }) {
                    results.push(result.clone())
                } else {
                    let fn_name = call.fn_name().to_owned();
                    let source = call.source().unwrap_or_default().to_owned();
                    let position = call.position();
                    let error = EvalAltResult::ErrorSystem(
                        "Invalid Answer".to_owned(),
                        Box::new(ArchetectError::GeneralError(if let Some(key) = key {
                            format!(
                                "'{}' was provided as an answer to '{}', but did not match any of the required options.",
                                answer, key.as_ref()
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
            }
            return Ok(results.into());
        } else {
            let fn_name = call.fn_name().to_owned();
            let source = call.source().unwrap_or_default().to_owned();
            let position = call.position();
            let error = EvalAltResult::ErrorSystem(
                "Invalid Answer Type".to_owned(),
                Box::new(ArchetectError::GeneralError(if let Some(key) = key {
                    format!(
                        "'{}' was provided as an answer to '{}', but must be an array of values or a comma-separated string.",
                        answer, key.as_ref()
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
    }

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
                return Ok(results.into());
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
                        key.as_ref()
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

    handle_result(result, optional)
}
