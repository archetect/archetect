use std::borrow::Cow;

use rhai::{Dynamic, EvalAltResult, Map, NativeCallContext};

use archetect_api::{CommandRequest, CommandResponse, PromptInfo, SelectPromptInfo};

use crate::errors::{ArchetectError, ArchetypeError};
use crate::runtime::context::RuntimeContext;
use crate::script::rhai::modules::prompt::get_optional_setting;
use crate::utils::{ArchetectRhaiSystemError, ArchetypeRhaiFunctionError, ArchetypeRhaiSystemError};

pub fn prompt<'a, K: Into<Cow<'a, str>>>(
    call: &NativeCallContext,
    message: &str,
    options: Vec<Dynamic>,
    runtime_context: &RuntimeContext,
    settings: &Map,
    key: Option<K>,
    answer: Option<&Dynamic>,
) -> Result<Dynamic, Box<EvalAltResult>> {
    let options = &options;

    if let Some(answer) = answer {
        for option in options {
            if option.to_string().as_str().to_lowercase() == answer.to_string().as_str().to_lowercase() {
                return Ok(option.clone());
            }
        }
        let requirement = "must match one of the required options";
        let error = ArchetypeError::answer_validation_error(answer.to_string(), message, key, requirement);
        return Err(ArchetypeRhaiFunctionError("Invalid Answer", call, error).into());
    };

    if runtime_context.headless() {
        return Err(Box::new(EvalAltResult::ErrorSystem(
            "Headless Mode Error".to_owned(),
            Box::new(ArchetectError::HeadlessNoDefault),
        )));
    }

    let options = options.iter().map(|v| v.to_string()).collect::<Vec<String>>();

    let mut prompt_info = SelectPromptInfo::new(message, options).with_optional(get_optional_setting(settings));

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
        prompt_info = prompt_info.with_placeholder(Some(help_message.to_string()));
    }

    runtime_context.request(CommandRequest::PromptForSelect(prompt_info.clone()));

    match runtime_context.response() {
        CommandResponse::String(answer) => {
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
            return Err(ArchetectRhaiSystemError("Prompt Error", ArchetectError::NakedError(error)).into());
        }
        response => {
            return Err(ArchetectRhaiSystemError("Prompt Error", ArchetectError::NakedError(format!("{:?}", response))).into());
        }
    }
}
