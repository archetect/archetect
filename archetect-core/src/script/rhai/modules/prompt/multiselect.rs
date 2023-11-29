use rhai::{Dynamic, EvalAltResult, Map, NativeCallContext};

use archetect_api::{CommandRequest, CommandResponse, MultiSelectPromptInfo};

use crate::errors::ArchetectError;
use crate::runtime::context::RuntimeContext;
use crate::script::rhai::modules::prompt::{create_error_from_call, get_optional_setting};

pub fn prompt<K: AsRef<str>>(
    call: NativeCallContext,
    message: &str,
    options: Vec<Dynamic>,
    runtime_context: &RuntimeContext,
    settings: &Map,
    key: Option<K>,
    answer: Option<&Dynamic>,
) -> Result<Dynamic, Box<EvalAltResult>> {

    let options = options.iter()
        .map(|v|v.to_string())
        .collect::<Vec<String>>();

    // Handle answers
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
            let error = EvalAltResult::ErrorSystem(
                "Invalid Answer".to_owned(),
                Box::new(ArchetectError::NakedError(if let Some(key) = key {
                    format!(
                        "'{}' was provided as an answer to Prompt: '{}' (key: '{}'), but must be an array of values or a comma-separated string.",
                        answer, message, key.as_ref()
                    )
                        .to_owned()
                } else {
                    format!(
                        "'{}' was provided as an answer to Prompt: '{}', but must be an array of values or a comma-separated string.",
                        answer, message
                    )
                        .to_owned()
                })),

            );
            return Err(Box::new(error));
        }
    }

    let mut prompt_info = MultiSelectPromptInfo::new(message, options.clone())
        .with_optional(get_optional_setting(settings))
        ;

    let mut validated_defaults = vec![];
    if let Some(defaults_with) = settings.get("defaults_with") {
        if let Some(defaults) = defaults_with.clone().try_cast::<Vec<String>>() {
            for default in defaults.iter() {
                if options.contains(default) {
                    // TODO: Error on invalid option
                    validated_defaults.push(default.to_owned());
                }
            }
            if runtime_context.headless() {
                return Ok(validated_defaults.into());
            } else {
                prompt_info = prompt_info.with_defaults(Some(validated_defaults));

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

    // if let Some(page_size) = settings.get("page_size") {
    //     if let Some(page_size) = page_size.clone().try_cast::<i64>() {
    //         prompt.page_size = page_size as usize;
    //     } else {
    //         warn!(
    //             "Invalid data type used for 'page_size': {}; should be an integer",
    //             page_size.type_name()
    //         );
    //     }
    // } else {
    //     prompt.page_size = 10;
    // }

    if let Some(placeholder) = settings.get("placeholder") {
        prompt_info = prompt_info.with_placeholder(Some(placeholder.to_string()));
    }

    if let Some(help_message) = settings.get("help") {
        prompt_info = prompt_info.with_help(Some(help_message.to_string()));
    }

    runtime_context.request(CommandRequest::PromptForMultiSelect(prompt_info));

    match runtime_context.responses().lock().unwrap().recv().expect("Error Receiving Response") {
        CommandResponse::MultiStringAnswer(answer) => {
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
