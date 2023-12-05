use rhai::{Dynamic, EvalAltResult, Map, NativeCallContext};

use archetect_api::{CommandRequest, CommandResponse, ListPromptInfo, PromptInfo};

use crate::errors::{ArchetypeScriptError, ArchetypeScriptErrorWrapper};
use crate::runtime::context::RuntimeContext;
use crate::script::rhai::modules::prompt::{cast_setting, parse_setting};

pub fn prompt<'a, K: AsRef<str> + Clone>(
    call: &NativeCallContext,
    message: &str,
    runtime_context: &RuntimeContext,
    settings: &Map,
    key: Option<K>,
    answer: Option<&Dynamic>,
) -> Result<Option<Vec<String>>, Box<EvalAltResult>> {
    let optional = cast_setting("optional", settings, message, key.clone())
        .map_err(|error| ArchetypeScriptErrorWrapper(call, error))?
        .unwrap_or_default();
    let min_items = parse_setting("min_items", settings, message, key.clone())
        .map_err(|error| ArchetypeScriptErrorWrapper(call, error))?;
    let max_items = parse_setting("max_items", settings, message, key.clone())
        .map_err(|error| ArchetypeScriptErrorWrapper(call, error))?;
    let placeholder = cast_setting("placeholder", settings, message, key.clone())
        .map_err(|error| ArchetypeScriptErrorWrapper(call, error))?;
    let help = cast_setting("help", settings, message, key.clone())
        .map_err(|error| ArchetypeScriptErrorWrapper(call, error))?
        .or_else(|| if optional { Some("<esc> for None".to_string()) } else { None })
        ;

    let mut prompt_info = ListPromptInfo::new(message)
        .with_optional(optional)
        .with_min_items(min_items)
        .with_max_items(max_items)
        .with_placeholder(placeholder)
        .with_help(help)
        ;

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

    if let Some(default_value) = settings.get("defaults_with") {
        if let Some(defaults) = default_value.clone().try_cast::<Vec<Dynamic>>() {
            let defaults = defaults.into_iter()
                .map(|v| v.to_string())
                .collect();
            if runtime_context.headless() {
                return Ok(Some(defaults));
            } else {
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

    runtime_context.request(CommandRequest::PromptForList(prompt_info.clone()));

    match runtime_context.response() {
        CommandResponse::Array(answer) => {
            return Ok(Some(answer.into()));
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
            let error = ArchetypeScriptError::unexpected_prompt_response(message, key, "Array of Strings", response);
            return Err(ArchetypeScriptErrorWrapper(call, error).into());
        }
    }
}
