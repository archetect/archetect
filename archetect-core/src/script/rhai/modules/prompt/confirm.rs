use rhai::{Dynamic, EvalAltResult, Map};

use archetect_api::{BoolPromptInfo, CommandRequest, CommandResponse, ValueSource};

use crate::errors::ArchetectError;
use crate::runtime::context::RuntimeContext;
use crate::script::rhai::modules::prompt::get_optional_setting;

// TODO: Better help messages
pub fn prompt<K: AsRef<str>>(
    message: &str,
    runtime_context: &RuntimeContext,
    settings: &Map,
    key: Option<K>,
    answer: Option<&Dynamic>,
) -> Result<Dynamic, Box<EvalAltResult>> {
    if let Some(answer) = answer {
        return get_boolean(answer.to_string().as_str(), message, key, ValueSource::Answer).map(|v| v.into());
    }

    let optional = get_optional_setting(settings);

    let mut prompt_info = BoolPromptInfo::new(message);

    if let Some(default_value) = settings.get("defaults_with") {
        let default = get_boolean(
            default_value.to_string().as_str(),
            message,
            key,
            ValueSource::DefaultsWith,
        )?;

        if runtime_context.headless() {
            return Ok(default.into());
        } else {
            prompt_info = prompt_info.with_default(Some(default));
        }
    }

    if runtime_context.headless() {
        return Err(Box::new(EvalAltResult::ErrorSystem(
            "Headless Mode".to_owned(),
            Box::new(ArchetectError::HeadlessNoDefault),
        )));
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

    runtime_context.request(CommandRequest::PromptForBool(prompt_info));

    match runtime_context.responses().lock().unwrap().recv().expect("Error Receiving Response") {
        CommandResponse::BoolAnswer(answer) => {
            return Ok(answer.into());
        }
        CommandResponse::NoneAnswer => {
            return Ok(Dynamic::UNIT);
        }
        CommandResponse::Error(error) => {
            let error = EvalAltResult::ErrorSystem("Prompt Error".to_string(), Box::new(ArchetectError::NakedError(error)));
            return Err(Box::new(error));
        }
        response => {
            let error = EvalAltResult::ErrorSystem("Invalid Answer Type".to_string(), Box::new(ArchetectError::NakedError(format!("{:?}", response))));
            return Err(Box::new(error));
        }
    }
}

fn get_boolean<K: AsRef<str>>(
    value: &str,
    prompt: &str,
    key: Option<K>,
    source: ValueSource,
) -> Result<bool, Box<EvalAltResult>> {
    match value.to_lowercase().as_str() {
        "y" | "yes" | "t" | "true" => Ok(true),
        "n" | "no" | "f" | "false" => Ok(false),
        _ => Err(Box::new(EvalAltResult::ErrorSystem(
            source.error_header(),
            Box::new(ArchetectError::NakedError(if let Some(key) = key {
                format!(
                    "'{}' was provided as {} to prompt: '{}' with Key: '{}', but must resemble a boolean",
                    value.to_string(),
                    source.description(),
                    prompt,
                    key.as_ref(),
                )
                .to_owned()
            } else {
                format!(
                    "'{}' was provided as {} to prompt: '{}', but must resemble a boolean",
                    value.to_string(),
                    source.description(),
                    prompt,
                )
                .to_owned()
            })),
        ))),
    }
}
