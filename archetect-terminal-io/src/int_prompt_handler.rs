use archetect_api::{ClientMessage, IntPromptInfo, PromptInfo, PromptInfoLengthRestrictions};
use archetect_validations::validate_int_size;
use inquire::validator::Validation;
use inquire::{InquireError, Text};

use crate::get_render_config;
use crate::responder::Responder;

pub fn handle_prompt_int(prompt_info: IntPromptInfo, responses: &dyn Responder) {
    let default_str = prompt_info.default().map(|v| v.to_string());
    let placeholder_str = prompt_info.placeholder().map(|v| v.to_string());
    let help_str = prompt_info.help().map(|v| v.to_string());
    let is_optional = prompt_info.optional();
    let min = prompt_info.min();
    let max = prompt_info.max();

    // Required prompts: Esc reprompts, Ctrl+C aborts. Optional prompts:
    // Esc skips (→ None), Ctrl+C aborts. Rebuild the prompt each iteration
    // so inquire redraws cleanly after a reprompt.
    loop {
        let mut prompt = Text::new(prompt_info.message()).with_render_config(get_render_config());
        prompt.default = default_str.as_deref();
        prompt.placeholder = placeholder_str.as_deref();
        prompt.help_message = help_str.as_deref();
        prompt = prompt.with_validator(move |input: &str| match validate(min, max, input) {
            Ok(_) => Ok(Validation::Valid),
            Err(message) => Ok(Validation::Invalid(message.into())),
        });

        let result = if is_optional {
            prompt.prompt_skippable()
        } else {
            prompt.prompt().map(Some)
        };

        match result {
            Ok(Some(answer)) => {
                responses.respond(ClientMessage::Integer(
                    answer.parse::<i64>().expect("Pre-validated"),
                ));
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

fn validate(min: Option<i64>, max: Option<i64>, input: &str) -> Result<(), String> {
    match input.parse::<i64>() {
        Ok(value) => validate_int_size(min, max, value),
        Err(_) => Err(format!("{} is not an 'int'", input)),
    }
}
