use rhai::{Dynamic, EvalAltResult, Map, NativeCallContext};

use archetect_api::{ClientMessage, PromptInfo, ScriptMessage, SelectPromptInfo};

use crate::Archetect;
use crate::archetype::render_context::RenderContext;
use crate::errors::{ArchetypeScriptError, ArchetypeScriptErrorWrapper};
use crate::script::rhai::modules::prompt_module::{cast_setting, extract_prompt_info, extract_prompt_info_pageable};

pub fn prompt<'a, K: AsRef<str> + Clone>(
    call: &NativeCallContext,
    message: &str,
    options: Vec<Dynamic>,
    archetect: &Archetect,
    render_context: &RenderContext,
    settings: &Map,
    key: Option<K>,
    answer: Option<&Dynamic>,
) -> Result<Option<String>, Box<EvalAltResult>> {
    let options = &options;
    let options = options.iter().map(|v| v.to_string()).collect::<Vec<String>>();
    let default = cast_setting("defaults_with", settings, message, key.clone())
        .map_err(|error| ArchetypeScriptErrorWrapper(call, error))?;

    let mut prompt_info = SelectPromptInfo::new(message, key, options.clone()).with_default(default.clone());

    extract_prompt_info(&mut prompt_info, settings).map_err(|error| ArchetypeScriptErrorWrapper(call, error))?;
    extract_prompt_info_pageable(&mut prompt_info, settings)
        .map_err(|error| ArchetypeScriptErrorWrapper(call, error))?;

    if let Some(answer) = answer {
        for option in options {
            if option.to_string().as_str().to_lowercase() == answer.to_string().as_str().to_lowercase() {
                return Ok(option.into());
            }
        }
        let requirement = "must match one of the required options";
        let error = ArchetypeScriptError::answer_validation_error(answer.to_string(), &prompt_info, requirement);
        return Err(ArchetypeScriptErrorWrapper(call, error).into());
    };

    if archetect.is_headless()
        || render_context.use_defaults_all()
        || render_context.use_defaults().contains(prompt_info.key().unwrap_or(""))
    {
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

    archetect.request(ScriptMessage::PromptForSelect(prompt_info.clone()))?;

    return match archetect.receive()? {
        ClientMessage::String(answer) => Ok(Some(answer)),
        ClientMessage::None => {
            if !prompt_info.optional() {
                let error = ArchetypeScriptError::answer_not_optional(&prompt_info);
                Err(ArchetypeScriptErrorWrapper(call, error).into())
            } else {
                Ok(None)
            }
        }
        ClientMessage::Error(error) => {
            Err(ArchetypeScriptErrorWrapper(call, ArchetypeScriptError::PromptError(error)).into())
        }
        ClientMessage::Abort => Err(Box::new(EvalAltResult::ErrorTerminated(Dynamic::UNIT, call.position()))),
        response => {
            let error = ArchetypeScriptError::unexpected_prompt_response(&prompt_info, "a String", response);
            Err(ArchetypeScriptErrorWrapper(call, error).into())
        }
    };
}
