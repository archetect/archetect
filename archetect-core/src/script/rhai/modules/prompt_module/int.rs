use rhai::{Dynamic, EvalAltResult, Map, NativeCallContext};

use archetect_api::{CommandRequest, CommandResponse, IntPromptInfo, PromptInfo, PromptInfoLengthRestrictions};
use archetect_validations::validate_int_size;

use crate::errors::{ArchetypeScriptError, ArchetypeScriptErrorWrapper};
use crate::Archetect;
use crate::archetype::render_context::RenderContext;
use crate::script::rhai::modules::prompt_module::{cast_setting, extract_prompt_info, extract_prompt_length_restrictions};

pub fn prompt_int<'a, K: AsRef<str> + Clone>(
    call: &NativeCallContext,
    message: &str,
    archetect: &Archetect,
    render_context: &RenderContext,
    settings: &Map,
    key: Option<K>,
    answer: Option<&Dynamic>,
) -> Result<Option<i64>, Box<EvalAltResult>> {
    let default = cast_setting("defaults_with", settings, message, key.clone())
        .map_err(|error| ArchetypeScriptErrorWrapper(call, error))?;

    let mut prompt_info = IntPromptInfo::new(message, key)
        .with_default(default)
        ;

    extract_prompt_info(&mut prompt_info, settings)
        .map_err(|error| ArchetypeScriptErrorWrapper(call, error))?;
    extract_prompt_length_restrictions(&mut prompt_info, settings)
        .map_err(|error| ArchetypeScriptErrorWrapper(call, error))?;

    if let Some(answer) = answer {
        return if let Some(answer) = answer.clone().try_cast::<i64>() {
            match validate_int_size(prompt_info.min(), prompt_info.max(), answer) {
                Ok(_) => Ok(Some(answer)),
                Err(error_message) => {
                    let error =
                        ArchetypeScriptError::answer_validation_error(answer.to_string(), &prompt_info, error_message);
                    return Err(ArchetypeScriptErrorWrapper(call, error).into());
                }
            }
        } else {
            let error = ArchetypeScriptError::answer_type_error(answer.to_string(), &prompt_info, "an Integer");
            Err(ArchetypeScriptErrorWrapper(call, error).into())
        };
    }

    if archetect.is_headless() || render_context.use_defaults_all() || render_context.use_defaults().contains(prompt_info.key().unwrap_or("")) {
        if let Some(default) = prompt_info.default() {
            return Ok(Some(default));
        } else if prompt_info.optional() {
            return Ok(None)
        }
        if archetect.is_headless() {
            let error = ArchetypeScriptError::headless_no_answer(&prompt_info);
            return Err(ArchetypeScriptErrorWrapper(call, error).into());
        }

    }

    archetect.request(CommandRequest::PromptForInt(prompt_info.clone()));

    match archetect.response() {
        CommandResponse::Integer(answer) => match validate_int_size(prompt_info.min(), prompt_info.max(), answer) {
            Ok(_) => Ok(Some(answer)),
            Err(error_message) => {
                let error =
                    ArchetypeScriptError::answer_validation_error(answer.to_string(), &prompt_info, error_message);
                return Err(ArchetypeScriptErrorWrapper(call, error).into());
            }
        },
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
        CommandResponse::Abort => {
            return Err(Box::new(EvalAltResult::Exit(Dynamic::UNIT, call.position())));
        },
        response => {
            let error = ArchetypeScriptError::unexpected_prompt_response(&prompt_info, "Int", response);
            return Err(ArchetypeScriptErrorWrapper(call, error).into());
        }
    }
}
