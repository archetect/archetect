use rhai::{Dynamic, EvalAltResult, Map, NativeCallContext};

use archetect_api::{CommandRequest, CommandResponse, EditorPromptInfo, PromptInfo, PromptInfoLengthRestrictions};
use archetect_validations::validate_text_length;

use crate::errors::{ArchetypeScriptError, ArchetypeScriptErrorWrapper};
use crate::Archetect;
use crate::script::rhai::modules::prompt_module::{cast_setting, extract_prompt_info, extract_prompt_length_restrictions};

pub fn prompt<'a, K: AsRef<str> + Clone>(
    call: &NativeCallContext,
    message: &str,
    settings: &Map,
    archetect: &Archetect,
    key: Option<K>,
    answer: Option<&Dynamic>,
) -> Result<Option<String>, Box<EvalAltResult>> {
    let default = cast_setting("defaults_with", settings, message, key.clone())
        .map_err(|error| ArchetypeScriptErrorWrapper(call, error))?;

    let mut prompt_info = EditorPromptInfo::new(message, key)
        .with_default(default)
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

    if archetect.is_headless() {
        if let Some(default) = prompt_info.default() {
            return Ok(Some(default));
        } else if prompt_info.optional() {
            return Ok(None);
        }
        let error = ArchetypeScriptError::headless_no_answer(&prompt_info);
        return Err(ArchetypeScriptErrorWrapper(call, error).into());
    }

    archetect.request(CommandRequest::PromptForEditor(prompt_info.clone()));

    match archetect.response() {
        CommandResponse::String(answer) => {
            match validate_text_length(prompt_info.min(), prompt_info.max(), &answer.to_string()) {
                Ok(_) => Ok(answer.into()),
                Err(error_message) => {
                    let error = ArchetypeScriptError::answer_validation_error(answer.to_string(), &prompt_info, error_message);
                    return Err(ArchetypeScriptErrorWrapper(call, error).into());
                }
            }
        }
        CommandResponse::None => {
            if !prompt_info.optional() {
                let error = ArchetypeScriptError::answer_not_optional(&prompt_info);
                return Err(ArchetypeScriptErrorWrapper(call, error).into());
            } else {
                return Ok(None);
            }
        }
        CommandResponse::Error(error) => {
            return Err(ArchetypeScriptErrorWrapper(call, ArchetypeScriptError::PromptError(error)).into());
        }
        response => {
            let error = ArchetypeScriptError::unexpected_prompt_response(&prompt_info, "a String", response);
            return Err(ArchetypeScriptErrorWrapper(call, error).into());
        }
    }
}
