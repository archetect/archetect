use std::borrow::Cow;

use rhai::{Dynamic, EvalAltResult, Map, NativeCallContext};

use archetect_api::{CommandRequest, CommandResponse, ListPromptInfo, PromptInfo};

use crate::errors::{ArchetypeScriptError, ArchetypeScriptErrorWrapper};
use crate::runtime::context::RuntimeContext;
use crate::script::rhai::modules::prompt::{get_optional_setting, parse_setting};

pub fn prompt<'a, K: Into<Cow<'a, str>>>(
    call: &NativeCallContext,
    message: &str,
    runtime_context: &RuntimeContext,
    settings: &Map,
    key: Option<K>,
    answer: Option<&Dynamic>,
) -> Result<Dynamic, Box<EvalAltResult>> {
    if let Some(answer) = answer {
        if let Some(answer) = answer.clone().try_cast::<String>() {
            let answers = answer
                .split(',')
                .map(|v| v.trim())
                .map(|v| v.to_owned())
                .collect::<Vec<String>>();
            return Ok(answers.into());
        }

        if let Some(answers) = answer.clone().try_cast::<Vec<Dynamic>>() {
            let answers = answers.iter().map(|v| v.to_string()).collect::<Vec<String>>();
            return Ok(answers.into());
        }

        let requirement = " an Array of Strings or a comma-separated String".to_string();
        let error = ArchetypeScriptError::answer_validation_error(answer.to_string(), message, key, requirement);
        return Err(ArchetypeScriptErrorWrapper(call, error).into());
    }

    let mut prompt_info = ListPromptInfo::new(message)
        .with_optional(get_optional_setting(settings))
        .with_min_items(parse_setting::<usize>("min_items", settings))
        .with_max_items(parse_setting::<usize>("max_items", settings));

    if let Some(default_value) = settings.get("defaults_with") {
        if let Some(defaults) = default_value.clone().try_cast::<Vec<Dynamic>>() {
            if runtime_context.headless() {
                return Ok(defaults.into());
            } else {
                let defaults = defaults.into_iter()
                    .map(|v| v.to_string())
                    .collect();
                prompt_info = prompt_info.with_defaults(Some(defaults));
            }
        } else {
            // TODO: Throw error about wrong type
        }
    }

    if runtime_context.headless() {
        let error = ArchetypeScriptError::headless_no_answer(message, key);
        return Err(ArchetypeScriptErrorWrapper(call, error).into());
    }

    if let Some(placeholder) = settings.get("placeholder") {
        prompt_info = prompt_info.with_placeholder(Some(placeholder.to_string()));
    }

    if let Some(help_message) = settings.get("help") {
        prompt_info = prompt_info.with_placeholder(Some(help_message.to_string()));
    }

    runtime_context.request(CommandRequest::PromptForList(prompt_info.clone()));

    match runtime_context.response() {
        CommandResponse::Array(answer) => {
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
            let error = ArchetypeScriptError::unexpected_prompt_response(message, key, "Array of Strings", response);
            return Err(ArchetypeScriptErrorWrapper(call, error).into());
        }
    }
}
