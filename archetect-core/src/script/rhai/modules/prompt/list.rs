use rhai::{Dynamic, EvalAltResult, Map, NativeCallContext};

use archetect_api::{CommandRequest, CommandResponse, ListPromptInfo};

use crate::errors::ArchetectError;
use crate::runtime::context::RuntimeContext;
use crate::script::rhai::modules::prompt::{get_optional_setting, parse_setting};

pub fn prompt<K: AsRef<str>>(
    call: NativeCallContext,
    message: &str,
    runtime_context: &RuntimeContext,
    settings: &Map,
    key: Option<K>,
    answer: Option<&Dynamic>,
) -> Result<Dynamic, Box<EvalAltResult>> {
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

        let fn_name = call.fn_name().to_owned();
        let source = call.source().unwrap_or_default().to_owned();
        let position = call.position();
        let error = EvalAltResult::ErrorSystem(
            "Invalid Answer Type".to_owned(),
            Box::new(ArchetectError::GeneralError(if let Some(key) = key {
                format!(
                    "'{}' was provided as an answer to Prompt: '{}' (key: '{}'), but must be an array of values or a comma-separated string.",
                    answer, message, key.as_ref()
                )
                    .to_owned()
            } else {
                format!(
                    "'{}' was provided as an answer to Prompt: '{}', but must be an array of values or a comma-separated string.",
                    answer, message
                ).to_owned()
            })),
        );
        return Err(Box::new(EvalAltResult::ErrorInFunctionCall(
            fn_name,
            source,
            Box::new(error),
            position,
        )));
    }

    let mut prompt_info = ListPromptInfo::new(message)
        .with_optional(get_optional_setting(settings))
        .with_min_items(parse_setting::<usize>("min_items", settings))
        .with_max_items(parse_setting::<usize>("max_items", settings))
        ;

    if let Some(default_value) = settings.get("defaults_with") {
        if let Some(defaults) = default_value.clone().try_cast::<Vec<String>>() {
            if runtime_context.headless() {
                return Ok(defaults.into());
            } else {
                prompt_info = prompt_info.with_defaults(Some(defaults));
            }
        } else {
            // TODO: Throw error about wrong type
        }
    }

    if runtime_context.headless() {
        let fn_name = call.fn_name().to_owned();
        let source = call.source().unwrap_or_default().to_owned();
        let position = call.position();
        let error = EvalAltResult::ErrorSystem(
            "Headless Mode Error".to_owned(),
            Box::new(ArchetectError::GeneralError(if let Some(key) = key {
                format!("{} for '{}'", message, key.as_ref(),).to_owned()
            } else {
                format!("{}", message).to_owned()
            })),
        );
        return Err(Box::new(EvalAltResult::ErrorInFunctionCall(
            fn_name,
            source,
            Box::new(error),
            position,
        )));
    }

    if let Some(placeholder) = settings.get("placeholder") {
        prompt_info = prompt_info.with_placeholder(Some(placeholder.to_string()));
    }

    if let Some(help_message) = settings.get("help") {
        prompt_info = prompt_info.with_placeholder(Some(help_message.to_string()));
    }

    runtime_context.request(CommandRequest::PromptForList(prompt_info));

    match runtime_context.response() {
        CommandResponse::MultiStringAnswer(answer) => {
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
