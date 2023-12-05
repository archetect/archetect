use rhai::{Dynamic, EvalAltResult, Map, NativeCallContext};

use archetect_api::{CommandRequest, CommandResponse, MultiSelectPromptInfo, PromptInfo};

use crate::errors::{ArchetypeScriptError, ArchetypeScriptErrorWrapper};
use crate::runtime::context::RuntimeContext;
use crate::script::rhai::modules::prompt::cast_setting;

pub fn prompt<'a, K: AsRef<str> + Clone>(
    call: &NativeCallContext,
    message: &str,
    options: Vec<Dynamic>,
    runtime_context: &RuntimeContext,
    settings: &Map,
    key: Option<K>,
    answer: Option<&Dynamic>,
) -> Result<Option<Vec<String>>, Box<EvalAltResult>> {
    let options = options.iter().map(|v| v.to_string()).collect::<Vec<String>>();
    let optional = cast_setting("optional", settings, message, key.clone())
        .map_err(|error| ArchetypeScriptErrorWrapper(call, error))?
        .unwrap_or_default();
    let placeholder = cast_setting("placeholder", settings, message, key.clone())
        .map_err(|error| ArchetypeScriptErrorWrapper(call, error))?;
    let help = cast_setting("help", settings, message, key.clone())
        .map_err(|error| ArchetypeScriptErrorWrapper(call, error))?
        .or_else(|| if optional { Some("<esc> for None".to_string()) } else { None })
        ;

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
                    let error = ArchetypeScriptError::answer_validation_error(answer, message, key, requirement);
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
                        ArchetypeScriptError::answer_validation_error(answer.to_string(), message, key, requirement);
                    return Err(ArchetypeScriptErrorWrapper(call, error).into());
                }
            }
            return Ok(Some(results));
        } else {
            let error = ArchetypeScriptError::answer_validation_error(
                answer.to_string(),
                message,
                key,
                "an Array of Strings or a comma-separated String",
            );
            return Err(ArchetypeScriptErrorWrapper(call, error).into());
        }
    }

    let mut prompt_info =
        MultiSelectPromptInfo::new(message, options.clone())
            .with_optional(optional)
            .with_placeholder(placeholder)
            .with_help(help)
        ;

    let mut validated_defaults = vec![];
    if let Some(defaults_with) = settings.get("defaults_with") {
        if let Some(defaults) = defaults_with.clone().try_cast::<Vec<Dynamic>>() {
            for default in defaults.iter() {
                if options.contains(&default.to_string()) {
                    // TODO: Error on invalid option
                    validated_defaults.push(default.to_string());
                }
            }
            if runtime_context.headless() {
                return Ok(validated_defaults.into());
            } else {
                prompt_info = prompt_info.with_defaults(Some(validated_defaults));
            }
        } else {
            let requirement = "must be an array of values";
            let error = ArchetypeScriptError::default_type_error(defaults_with.to_string(), message, key, requirement);
            return Err(ArchetypeScriptErrorWrapper(call, error).into());
        }
    }

    if runtime_context.headless() {
        let error = ArchetypeScriptError::headless_no_answer(message, key);
        return Err(ArchetypeScriptErrorWrapper(call, error).into());
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

    runtime_context.request(CommandRequest::PromptForMultiSelect(prompt_info.clone()));

    match runtime_context.response() {
        CommandResponse::Array(answer) => {
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
            let error = ArchetypeScriptError::unexpected_prompt_response(message, key, "Array of Strings", response);
            return Err(ArchetypeScriptErrorWrapper(call, error).into());
        }
    }
}
