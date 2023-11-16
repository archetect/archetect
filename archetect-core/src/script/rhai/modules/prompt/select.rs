use log::warn;
use rhai::{Dynamic, EvalAltResult, Map, NativeCallContext};

use inquire::Select;

use crate::errors::ArchetectError;
use crate::runtime::context::RuntimeContext;
use crate::script::rhai::modules::prompt::{get_optional_setting, get_render_config, handle_result};

pub fn prompt<K: AsRef<str>>(
    call: NativeCallContext,
    message: &str,
    options: Vec<Dynamic>,
    runtime_context: &RuntimeContext,
    settings: &Map,
    key: Option<K>,
    answer: Option<&Dynamic>,
) -> Result<Dynamic, Box<EvalAltResult>> {
    let optional = get_optional_setting(settings);

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

    let default = if let Some(defaults_with) = settings.get("defaults_with") {
        let default = options
            .iter()
            .position(|item| item.to_string().as_str() == defaults_with.to_string().as_str());
        if default.is_none() {
            warn!("A 'defaults_with' was set, but did not match any of the options.")
        }
        default
    } else {
        None
    };

    let mut prompt = Select::new(message, options.to_vec())
        .with_render_config(get_render_config())
        ;

    if let Some(default) = default {
        if runtime_context.headless() {
            return Ok(options.get(default).unwrap().clone());
        }
        prompt.starting_cursor = default;
    }

    if runtime_context.headless() {
        return Err(Box::new(EvalAltResult::ErrorSystem(
            "Headless Mode Error".to_owned(),
            Box::new(ArchetectError::HeadlessNoDefault),
        )));
    }

    if let Some(page_size) = settings.get("page_size") {
        if let Some(page_size) = page_size.clone().try_cast::<i64>() {
            prompt.page_size = page_size as usize;
        } else {
            warn!(
                "Invalid data type used for 'page_size': {}; should be an integer",
                page_size.type_name()
            );
        }
    } else {
        prompt.page_size = 10;
    }

    if let Some(help_message) = settings.get("help") {
        prompt.help_message = Some(help_message.to_string());
    }

    handle_result(prompt.prompt(), optional)
}
