use rhai::{Dynamic, EvalAltResult, Map, NativeCallContext};

use archetect_api::{ScriptMessage, ClientMessage, PromptInfo, PromptInfoLengthRestrictions, TextPromptInfo};
use archetect_validations::validate_text_length;

use crate::errors::{ArchetypeScriptError, ArchetypeScriptErrorWrapper};
use crate::Archetect;
use crate::archetype::render_context::RenderContext;
use crate::script::rhai::modules::prompt_module::{cast_setting, extract_prompt_info, extract_prompt_length_restrictions};

pub fn prompt<'a, K: AsRef<str> + Clone>(
    call: &NativeCallContext,
    message: &str,
    settings: &Map,
    archetect: &Archetect,
    render_context: &RenderContext,
    key: Option<K>,
    answer: Option<&Dynamic>,
) -> Result<Option<String>, Box<EvalAltResult>> {
    let defaults_with = cast_setting("defaults_with", settings, message, key.clone())
        .map_err(|error| ArchetypeScriptErrorWrapper(call, error))?;

    let mut prompt_info = TextPromptInfo::new(message, key)
        .with_default(defaults_with)
        ;

    extract_prompt_info(&mut prompt_info, settings)
        .map_err(|error| ArchetypeScriptErrorWrapper(call, error))?;
    extract_prompt_length_restrictions(&mut prompt_info, settings)
        .map_err(|error| ArchetypeScriptErrorWrapper(call, error))?;

    if let Some(answer) = answer {
        return if let Some(answer) = answer.clone().try_cast::<String>() {
            match validate_text_length(prompt_info.min(), prompt_info.max(), &answer.to_string()) {
                Ok(_) => Ok(answer.into()),
                Err(error_message) => {
                    let error = ArchetypeScriptError::answer_validation_error(answer.to_string(), &prompt_info, error_message);
                    return Err(ArchetypeScriptErrorWrapper(call, error).into());
                }
            }
        } else {
            let error = ArchetypeScriptError::answer_type_error(answer.to_string(), &prompt_info, "a String");
            Err(ArchetypeScriptErrorWrapper(call, error).into())
        }
    }

    if archetect.is_headless() || render_context.use_defaults_all() || render_context.use_defaults().contains(prompt_info.key().unwrap_or("")) {
        if let Some(default) = prompt_info.default() {
            return Ok(Some(default));
        } else if prompt_info.optional() {
            return Ok(None);
        }
        if archetect.is_headless() {
            let error = ArchetypeScriptError::headless_no_answer(&prompt_info);
            return Err(ArchetypeScriptErrorWrapper(call, error).into());
        }
    }


    archetect.request(ScriptMessage::PromptForText(prompt_info.clone()));

    match archetect.receive() {
        ClientMessage::String(answer) => {
            match validate_text_length(prompt_info.min(), prompt_info.max(), &answer.to_string()) {
                Ok(_) => Ok(answer.into()),
                Err(error_message) => {
                    let error = ArchetypeScriptError::answer_validation_error(answer.to_string(), &prompt_info, error_message);
                    return Err(ArchetypeScriptErrorWrapper(call, error).into());
                }
            }
        }
        ClientMessage::None => {
            if !prompt_info.optional() {
                let error = ArchetypeScriptError::answer_not_optional(&prompt_info);
                return Err(ArchetypeScriptErrorWrapper(call, error).into());
            } else {
                return Ok(None);
            }
        }
        ClientMessage::Error(error) => {
            return Err(ArchetypeScriptErrorWrapper(call, ArchetypeScriptError::PromptError(error)).into());
        }
        ClientMessage::Abort => {
            return Err(Box::new(EvalAltResult::ErrorTerminated(Dynamic::UNIT, call.position())));
        },
        response => {
            let error = ArchetypeScriptError::unexpected_prompt_response(&prompt_info, "a String", response);
            return Err(ArchetypeScriptErrorWrapper(call, error).into());
        }
    }
}
