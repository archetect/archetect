use std::borrow::Cow;

use rhai::{Dynamic, EvalAltResult, Map, NativeCallContext};

use archetect_api::{CommandRequest, CommandResponse, PromptInfo, SelectPromptInfo};

use crate::errors::{ArchetectError, ArchetypeScriptError, ArchetypeScriptErrorWrapper};
use crate::runtime::context::RuntimeContext;
use crate::script::rhai::modules::prompt::get_optional_setting;

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
        let error = ArchetypeScriptError::answer_validation_error(answer.to_string(), message, key, requirement);
        return Err(ArchetypeScriptErrorWrapper(call, error).into());
    };

    if runtime_context.headless() {
        return Err(Box::new(EvalAltResult::ErrorSystem(
            "Headless Mode Error".to_owned(),
            Box::new(ArchetectError::HeadlessNoDefault),
        )));
    }

    let options = options.iter().map(|v| v.to_string()).collect::<Vec<String>>();

    let mut prompt_info = SelectPromptInfo::new(message, options).with_optional(get_optional_setting(settings));

    if let Some(default_value) = settings.get("defaults_with") {
        if let Some(default) = default_value.clone().try_cast::<String>() {
            if runtime_context.headless() {
                return Ok(default.into());
            } else {
                prompt_info = prompt_info.with_default(Some(default));
            }
        } else {
            // TODO: Throw error about wrong type
        }
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
        prompt_info = prompt_info.with_placeholder(Some(help_message.to_string()));
    }

    runtime_context.request(CommandRequest::PromptForSelect(prompt_info.clone()));

    match runtime_context.response() {
        CommandResponse::String(answer) => {
            return Ok(answer.into());
        }
        CommandResponse::None => {
            if !prompt_info.optional() {
                let error = ArchetypeScriptError::answer_not_optional(message, key);
                return Err(ArchetypeScriptErrorWrapper(call, error).into());
            } else {
                return Ok(Dynamic::UNIT);
            }
        }
        CommandResponse::Error(error) => {
            return Err(ArchetypeScriptErrorWrapper(call, ArchetypeScriptError::PromptError(error)).into());
        }
        response => {
            let error = ArchetypeScriptError::unexpected_prompt_response(message, key, "a String", response);
            return Err(ArchetypeScriptErrorWrapper(call, error).into());
        }
    }
}
