use rhai::{Dynamic, EvalAltResult, Map};

use inquire::Confirm;

use crate::errors::ArchetectError;
use crate::runtime::context::RuntimeContext;
use crate::script::rhai::modules::prompt::{get_optional_setting, get_render_config, handle_result};

// TODO: Better help messages
pub fn prompt<K: AsRef<str>>(
    message: &str,
    runtime_context: &RuntimeContext,
    settings: &Map,
    key: Option<K>,
    answer: Option<&Dynamic>,
) -> Result<Dynamic, Box<EvalAltResult>> {
    let optional = get_optional_setting(settings);

    if let Some(answer) = answer {
        return get_boolean(answer.to_string().as_str(), message, key, ValueSource::Answer).map(|v| v.into());
    }

    let mut prompt = Confirm::new(message).with_render_config(get_render_config());

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
            prompt.default = Some(default);
        }
    }

    if runtime_context.headless() {
        return Err(Box::new(EvalAltResult::ErrorSystem(
            "Headless Mode".to_owned(),
            Box::new(ArchetectError::HeadlessNoDefault),
        )));
    }

    if let Some(placeholder) = settings.get("placeholder") {
        prompt.placeholder = Some(placeholder.to_string());
    }

    if let Some(help_message) = settings.get("help") {
        prompt.help_message = Some(help_message.to_string());
    } else {
        if optional {
            prompt.help_message = Some("<esc> for None".into());
        }
    }

    let validator = |ans: &str| {
        if ans.len() > 5 {
            return Err(());
        }

        get_boolean::<&str>(ans, message, None, ValueSource::Value).map_err(|_| ())
    };

    prompt.parser = &validator;

    handle_result(prompt.prompt(), optional)
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

enum ValueSource {
    Answer,
    DefaultsWith,
    Value,
}

impl ValueSource {
    fn error_header(&self) -> String {
        match self {
            ValueSource::Answer => "Answer Error".to_string(),
            ValueSource::DefaultsWith => "defaults_with Error".to_string(),
            ValueSource::Value => "Value Error".to_string(),
        }
    }

    fn description(&self) -> String {
        match self {
            ValueSource::Answer => "an answer".to_string(),
            ValueSource::DefaultsWith => "a defaults_with".to_string(),
            ValueSource::Value => "a value".to_string(),
        }
    }
}
