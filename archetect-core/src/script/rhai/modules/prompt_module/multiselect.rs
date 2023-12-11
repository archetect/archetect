use rhai::{Dynamic, EvalAltResult, Map, NativeCallContext};

use archetect_api::{CommandRequest, CommandResponse, MultiSelectPromptInfo, PromptInfo};

use crate::errors::{ArchetypeScriptError, ArchetypeScriptErrorWrapper};
use crate::Archetect;
use crate::archetype::render_context::RenderContext;
use crate::script::rhai::modules::prompt_module::{extract_prompt_info, extract_prompt_info_pageable, extract_prompt_items_restrictions};

pub fn prompt<'a, K: AsRef<str> + Clone>(
    call: &NativeCallContext,
    message: &str,
    options: Vec<Dynamic>,
    archetect: &Archetect,
    render_context: &RenderContext,
    settings: &Map,
    key: Option<K>,
    answer: Option<&Dynamic>,
) -> Result<Option<Vec<String>>, Box<EvalAltResult>> {
    let options = options.iter().map(|v| v.to_string()).collect::<Vec<String>>();

    let mut prompt_info = MultiSelectPromptInfo::new(message, key, options.clone());

    extract_prompt_info(&mut prompt_info, settings)
        .map_err(|error| ArchetypeScriptErrorWrapper(call, error))?;
    extract_prompt_items_restrictions(&mut prompt_info, settings)
        .map_err(|error| ArchetypeScriptErrorWrapper(call, error))?;
    extract_prompt_info_pageable(&mut prompt_info, settings)
        .map_err(|error| ArchetypeScriptErrorWrapper(call, error))?;

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
                    let error = ArchetypeScriptError::answer_validation_error(answer, &prompt_info, requirement);
                    return Err(ArchetypeScriptErrorWrapper(call, error).into());
                }
            }

            return Ok(Some(results));
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
                    let error =
                        ArchetypeScriptError::answer_validation_error(answer.to_string(), &prompt_info, requirement);
                    return Err(ArchetypeScriptErrorWrapper(call, error).into());
                }
            }
            return Ok(Some(results));
        } else {
            let error = ArchetypeScriptError::answer_validation_error(
                answer.to_string(),
                &prompt_info,
                "an Array of Strings or a comma-separated String",
            );
            return Err(ArchetypeScriptErrorWrapper(call, error).into());
        }
    }

    let mut validated_defaults = vec![];
    if let Some(defaults) = settings.get("defaults_with") {
        if let Some(defaults) = defaults.clone().try_cast::<Vec<Dynamic>>() {
            for default in defaults.iter() {
                if options.contains(&default.to_string()) {
                    // TODO: Error on invalid option
                    validated_defaults.push(default.to_string());
                }
            }
            prompt_info.set_defaults(Some(validated_defaults));
        } else {
            let requirement = "an Array of Strings";
            let error = ArchetypeScriptError::default_type_error(defaults.to_string(), &prompt_info, requirement);
            return Err(ArchetypeScriptErrorWrapper(call, error).into());
        }
    }

    if archetect.is_headless() || render_context.defaults_all() || render_context.defaults().contains(prompt_info.key().unwrap_or("")) {
        if let Some(default) = prompt_info.defaults() {
            return Ok(Some(default));
        } else if prompt_info.optional() {
            return Ok(None);
        } else {
            // TODO: Validate empty list
            return Ok(vec![].into())
        }
    }

    archetect.request(CommandRequest::PromptForMultiSelect(prompt_info.clone()));

    match archetect.response() {
        CommandResponse::Array(answer) => {
            return Ok(Some(answer));
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
            let error = ArchetypeScriptError::unexpected_prompt_response(&prompt_info, "Array of Strings", response);
            return Err(ArchetypeScriptErrorWrapper(call, error).into());
        }
    }
}
