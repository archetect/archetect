use rhai::{Dynamic, EvalAltResult, Map, NativeCallContext};

use archetect_api::{CommandRequest, CommandResponse, SelectPromptInfo};

use crate::errors::ArchetectError;
use crate::runtime::context::RuntimeContext;
use crate::script::rhai::modules::prompt::get_optional_setting;

pub fn prompt<K: AsRef<str>>(
    call: NativeCallContext,
    message: &str,
    options: Vec<Dynamic>,
    runtime_context: &RuntimeContext,
    settings: &Map,
    key: Option<K>,
    answer: Option<&Dynamic>,
) -> Result<Dynamic, Box<EvalAltResult>> {

    let options = &options;

    if let Some(answer) = answer {
        for option in options {
            if option.to_string().as_str().to_lowercase() == answer.to_string().as_str().to_lowercase() {
                return Ok(option.clone());
            }
        }

        let fn_name = call.fn_name().to_owned();
        let source = call.source().unwrap_or_default().to_owned();
        let position = call.position();
        let error = EvalAltResult::ErrorSystem(
            "Invalid Answer".to_owned(),
            Box::new(ArchetectError::GeneralError(if let Some(key) = key {
                format!(
                    "'{}' was provided as an answer to '{}', but did not match any of the required options.",
                    answer, key.as_ref()
                )
                .to_owned()
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
    };

    if runtime_context.headless() {
        return Err(Box::new(EvalAltResult::ErrorSystem(
            "Headless Mode Error".to_owned(),
            Box::new(ArchetectError::HeadlessNoDefault),
        )));
    }


    let options = options.iter().map(|v|v.to_string())
        .collect::<Vec<String>>();

    let mut prompt_info = SelectPromptInfo::new(message, options)
        .with_optional(get_optional_setting(settings))
        ;

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

    if let Some(placeholder) = settings.get("placeholder") {
        prompt_info = prompt_info.with_placeholder(Some(placeholder.to_string()));
    }

    if let Some(help_message) = settings.get("help") {
        prompt_info = prompt_info.with_placeholder(Some(help_message.to_string()));
    }

    runtime_context.request(CommandRequest::PromptForSelect(prompt_info));

    match runtime_context.response() {
        CommandResponse::StringAnswer(answer) => {
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
