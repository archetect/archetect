use archetect_api::{BoolPromptInfo, ClientMessage, PromptInfo, ValueSource};
use inquire::{Confirm, InquireError};

use crate::get_render_config;
use crate::responder::Responder;

pub fn handle_prompt_bool(prompt_info: BoolPromptInfo, responses: &dyn Responder) {
    let placeholder_str = prompt_info.placeholder().map(|v| v.to_string());
    let help_str = prompt_info.help().map(|v| v.to_string());
    let is_optional = prompt_info.optional();
    let default = prompt_info.default();
    let message = prompt_info.message().to_string();

    let parser = move |ans: &str| {
        if ans.len() > 5 {
            return Err(());
        }
        get_boolean::<&str>(ans, &message, None, ValueSource::Value).map_err(|_| ())
    };

    // Required: Esc reprompts, Ctrl+C aborts. Optional: Esc skips, Ctrl+C aborts.
    loop {
        let mut prompt = Confirm::new(prompt_info.message()).with_render_config(get_render_config());
        prompt.default = default;
        prompt.placeholder = placeholder_str.as_deref();
        prompt.help_message = help_str.as_deref();
        prompt.parser = &parser;

        let result = if is_optional {
            prompt.prompt_skippable()
        } else {
            prompt.prompt().map(Some)
        };

        match result {
            Ok(Some(answer)) => {
                responses.respond(ClientMessage::Boolean(answer));
                return;
            }
            Ok(None) => {
                responses.respond(ClientMessage::None);
                return;
            }
            Err(InquireError::OperationCanceled) if !is_optional => continue,
            Err(InquireError::OperationCanceled) => {
                responses.respond(ClientMessage::None);
                return;
            }
            Err(InquireError::OperationInterrupted) => {
                responses.respond(ClientMessage::Abort);
                return;
            }
            Err(error) => {
                responses.respond(ClientMessage::Error(error.to_string()));
                return;
            }
        }
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
