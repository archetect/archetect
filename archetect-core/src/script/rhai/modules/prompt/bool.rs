use rhai::{Dynamic, EvalAltResult, Map, NativeCallContext};

use archetect_api::{BoolPromptInfo, CommandRequest, CommandResponse, PromptInfo};

use crate::errors::{ArchetypeScriptError, ArchetypeScriptErrorWrapper};
use crate::runtime::context::RuntimeContext;
use crate::script::rhai::modules::prompt::cast_setting;

// TODO: Better help messages
pub fn prompt<'a, K: AsRef<str> + Clone>(
    call: &NativeCallContext,
    message: &str,
    runtime_context: &RuntimeContext,
    settings: &Map,
    key: Option<K>,
    answer: Option<&Dynamic>,
) -> Result<Dynamic, Box<EvalAltResult>> {
    if let Some(answer) = answer {
        return match get_boolean(answer.to_string().as_str()) {
            Ok(value) => Ok(value.into()),
            Err(_) => {
                let error = ArchetypeScriptError::answer_validation_error(
                    answer.to_string(),
                    message,
                    key,
                    "must resemble a boolean",
                );
                return Err(ArchetypeScriptErrorWrapper(call, error).into());
            }
        };
    }

    let optional = cast_setting("optional", settings, message, key.clone())
        .map_err(|error| ArchetypeScriptErrorWrapper(call, error))?
        .unwrap_or_default();
    let placeholder = cast_setting("placeholder", settings, message, key.clone())
        .map_err(|error| ArchetypeScriptErrorWrapper(call, error))?;
    let help = cast_setting("help", settings, message, key.clone())
        .map_err(|error| ArchetypeScriptErrorWrapper(call, error))?
        .or_else(|| if optional { Some("<esc> for None".to_string()) } else { None })
        ;

    let mut prompt_info = BoolPromptInfo::new(message)
        .with_optional(optional)
        .with_placeholder(placeholder)
        .with_help(help)
        ;

    if let Some(default_value) = settings.get("defaults_with") {
        match get_boolean(default_value.to_string().as_str()) {
            Ok(default) => {
                if runtime_context.headless() {
                    return Ok(default.into());
                } else {
                    prompt_info = prompt_info.with_default(Some(default));
                }
            }
            Err(_) => {
                let error = ArchetypeScriptError::default_validation_error(
                    default_value.to_string(),
                    message,
                    key,
                    "a Boolean",
                );
                return Err(ArchetypeScriptErrorWrapper(call, error).into());
            }
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
        prompt_info = prompt_info.with_help(Some(help_message.to_string()));
    } else {
        if optional {
            prompt_info = prompt_info.with_help(Some("<esc> for None".to_string()));
        }
    }

    runtime_context.request(CommandRequest::PromptForBool(prompt_info.clone()));

    match runtime_context.response() {
        CommandResponse::Boolean(answer) => {
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
            let error = ArchetypeScriptError::unexpected_prompt_response(message, key, "a Boolean", response);
            return Err(ArchetypeScriptErrorWrapper(call, error).into());
        }
    }
}

fn get_boolean<V: AsRef<str>>(value: V) -> Result<bool, ()> {
    match value.as_ref().to_lowercase().as_str() {
        "y" | "yes" | "t" | "true" => Ok(true),
        "n" | "no" | "f" | "false" => Ok(false),
        _ => Err(()),
    }
}
