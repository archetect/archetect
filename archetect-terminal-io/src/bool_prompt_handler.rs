use archetect_api::{BoolPromptInfo, ClientMessage, ValueSource};
use inquire::{Confirm, InquireError};

use crate::get_render_config;
use crate::responder::Responder;

pub fn handle_prompt_bool(prompt_info: BoolPromptInfo, responses: &dyn Responder) {
    let placeholder_str = prompt_info.placeholder().map(|v| v.to_string());
    let help_str = prompt_info.help().map(|v| v.to_string());

    let mut prompt = Confirm::new(prompt_info.message()).with_render_config(get_render_config());
    prompt.default = prompt_info.default();
    prompt.placeholder = placeholder_str.as_deref();
    prompt.help_message = help_str.as_deref();

    let prompt_info = prompt_info.clone();
    let parser = |ans: &str| {
        if ans.len() > 5 {
            return Err(());
        }
        get_boolean::<&str>(ans, prompt_info.message(), None, ValueSource::Value).map_err(|_| ())
    };
    prompt.parser = &parser;
    match prompt.prompt_skippable() {
        Ok(answer) => {
            if let Some(answer) = answer {
                responses.respond(ClientMessage::Boolean(answer));
            } else {
                responses.respond(ClientMessage::None);
            }
        }
        Err(error) => match error {
            InquireError::OperationCanceled | InquireError::OperationInterrupted => {
                responses.respond(ClientMessage::Abort);
            }
            _ => {
                responses.respond(ClientMessage::Error(error.to_string()));
            }
        },
    }
}

fn get_boolean<K: AsRef<str>>(
    value: &str,
    prompt: &str,
    key: Option<K>,
    source: ValueSource,
) -> Result<bool, String> {
    match value.to_lowercase().as_str() {
        "y" | "yes" | "t" | "true" => Ok(true),
        "n" | "no" | "f" | "false" => Ok(false),
        _ => Err(match key {
            Some(key) => format!(
                "'{}' was provided as {} to prompt: '{}' with Key: '{}', but must resemble a boolean",
                value, source.description(), prompt, key.as_ref(),
            ),
            None => format!(
                "'{}' was provided as {} to prompt: '{}', but must resemble a boolean",
                value, source.description(), prompt,
            ),
        }),
    }
}
