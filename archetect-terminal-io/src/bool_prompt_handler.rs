

use archetect_inquire::{Confirm, InquireError};

use archetect_api::{BoolPromptInfo, ClientMessage, ValueSource};
use crate::responder::Responder;

use crate::get_render_config;

pub fn handle_prompt_bool(prompt_info: BoolPromptInfo, responses: &dyn Responder) {
    let mut prompt = Confirm::new(prompt_info.message()).with_render_config(get_render_config());
    let default = prompt_info.default();
    prompt.default = default;
    prompt.placeholder = prompt_info.placeholder().map(|v| v.to_string());
    if prompt_info.help().is_some() {
        prompt.help_message = prompt_info.help().map(|v| v.to_string());
    }
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
        Err(error) => {
            match error {
                InquireError::OperationCanceled | InquireError::OperationInterrupted => {
                   responses.respond(ClientMessage::Abort);
                }
                _ => {
                    responses.respond(ClientMessage::Error(error.to_string()));
                }
            }
        }
    }
}

fn get_boolean<K: AsRef<str>>(value: &str, prompt: &str, key: Option<K>, source: ValueSource) -> Result<bool, String> {
    match value.to_lowercase().as_str() {
        "y" | "yes" | "t" | "true" => Ok(true),
        "n" | "no" | "f" | "false" => Ok(false),
        _ => Err(match key {
            Some(key) => {
                format!(
                    "'{}' was provided as {} to prompt: '{}' with Key: '{}', but must resemble a boolean",
                    value.to_string(),
                    source.description(),
                    prompt,
                    key.as_ref(),
                )
            }
            None => {
                format!(
                    "'{}' was provided as {} to prompt: '{}', but must resemble a boolean",
                    value.to_string(),
                    source.description(),
                    prompt,
                )
            }
        }),
    }
}
