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
) -> Result<Option<bool>, Box<EvalAltResult>> {
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
    let defaults_with = cast_setting("defaults_with", settings, message, key.clone())
        .map_err(|error| ArchetypeScriptErrorWrapper(call, error))?;

    let prompt_info = BoolPromptInfo::new(message)
        .with_optional(optional)
        .with_placeholder(placeholder)
        .with_help(help)
        .with_default(defaults_with.clone())
        ;

    if runtime_context.headless() {
        if let Some(default) = defaults_with {
            return Ok(Some(default));
        } else if optional {
            return Ok(None)
        }
        let error = ArchetypeScriptError::headless_no_answer(message, key);
        return Err(ArchetypeScriptErrorWrapper(call, error).into());
    }



    runtime_context.request(CommandRequest::PromptForBool(prompt_info.clone()));

    match runtime_context.response() {
        CommandResponse::Boolean(answer) => {
            return Ok(Some(answer));
        }
        CommandResponse::None => {
            if !prompt_info.optional() {
                let error = ArchetypeScriptError::answer_not_optional(message, key);
                return Err(ArchetypeScriptErrorWrapper(call, error).into());
            } else {
                return Ok(None);
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
