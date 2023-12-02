use std::borrow::Cow;
use rhai::{Dynamic, EvalAltResult, Map, NativeCallContext};

use archetect_api::{CommandRequest, CommandResponse, MultiSelectPromptInfo, PromptInfo};

use crate::errors::{ArchetectError, ArchetypeError};
use crate::runtime::context::RuntimeContext;
use crate::script::rhai::modules::prompt::get_optional_setting;
use crate::utils::{ArchetectRhaiFunctionError, ArchetypeRhaiFunctionError, ArchetypeRhaiSystemError};

pub fn prompt<'a, K: Into<Cow<'a, str>>>(
    call: &NativeCallContext,
    message: &str,
    options: Vec<Dynamic>,
    runtime_context: &RuntimeContext,
    settings: &Map,
    key: Option<K>,
    answer: Option<&Dynamic>,
) -> Result<Dynamic, Box<EvalAltResult>> {
    let options = options.iter().map(|v| v.to_string()).collect::<Vec<String>>();

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
                    let requirement = "must match one of the required options";
                    let error = ArchetypeError::answer_validation_error(answer, message, key, requirement);
                    return Err(ArchetypeRhaiFunctionError("Invalid Answer", call, error).into());
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
                    let requirement = "must match one of the required options";
                    let error = ArchetypeError::answer_validation_error(answer.to_string(), message, key, requirement);
                    return Err(ArchetypeRhaiFunctionError("Invalid Answer", call, error).into());
                }
            }
            return Ok(results.into());
        } else {
            let error = ArchetypeError::answer_validation_error(
                answer.to_string(),
                message,
                key,
                "must be an array of values or a comma-separated string",
            );
            return Err(ArchetypeRhaiFunctionError("Invalid Answer", call, error).into());
        }
    }

    let mut prompt_info =
        MultiSelectPromptInfo::new(message, options.clone()).with_optional(get_optional_setting(settings));

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
            let error = ArchetectError::GeneralError(if let Some(key) = key {
                format!(
                    "'{}' ({}) was provided as a default for '{}', but must be an array of values.",
                    defaults_with,
                    defaults_with.type_name(),
                    key.into(),
                )
                .to_owned()
            } else {
                message.to_string()
            });
            return Err(ArchetectRhaiFunctionError("Invalid Default", call, error).into());
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

    runtime_context.request(CommandRequest::PromptForMultiSelect(prompt_info.clone()));

    match runtime_context.response() {
        CommandResponse::Array(answer) => {
            return Ok(answer.into());
        }
        CommandResponse::None => {
            if !prompt_info.optional() {
                let error = ArchetypeError::answer_not_optional(message, key);
                return Err(ArchetypeRhaiSystemError("Required", error).into());
            } else {
                return Ok(Dynamic::UNIT);
            }
        }
        CommandResponse::Error(error) => {
            let error =
                EvalAltResult::ErrorSystem("Prompt Error".to_string(), Box::new(ArchetectError::NakedError(error)));
            return Err(Box::new(error));
        }
        response => {
            let error = EvalAltResult::ErrorSystem(
                "Invalid Answer Type".to_string(),
                Box::new(ArchetectError::NakedError(format!("{:?}", response))),
            );
            return Err(Box::new(error));
        }
    }
}
