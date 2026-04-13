use archetect_api::{ClientMessage, PromptInfo, PromptInfoLengthRestrictions, TextPromptInfo};
use archetect_validations::validate_text_length;
use inquire::validator::Validation;
use inquire::{InquireError, Text};

use crate::get_render_config;
use crate::responder::Responder;

pub fn handle_prompt_text(prompt_info: TextPromptInfo, responses: &dyn Responder) {
    let default_str = prompt_info.default().map(|v| v.to_string());
    let placeholder_str = prompt_info.placeholder().map(|v| v.to_string());
    let help_str = prompt_info.help().map(|v| v.to_string());

    let mut prompt = Text::new(prompt_info.message()).with_render_config(get_render_config());
    prompt.default = default_str.as_deref();
    prompt.placeholder = placeholder_str.as_deref();
    prompt.help_message = help_str.as_deref();

    let min = prompt_info.min();
    let max = prompt_info.max();
    let is_optional = prompt_info.optional();
    // Empty input is a skip. For required prompts it must be rejected —
    // inquire will auto-reprompt on Invalid, which matches the user
    // expectation: "reprompt on escape/empty; abort on Ctrl+C".
    let validator = move |input: &str| {
        if !is_optional && input.is_empty() {
            return Ok(Validation::Invalid("Answer is required.".into()));
        }
        match validate_text_length(min, max, input) {
            Ok(_) => Ok(Validation::Valid),
            Err(message) => Ok(Validation::Invalid(message.into())),
        }
    };
    prompt = prompt.with_validator(validator);

    if is_optional {
        match prompt.prompt_skippable() {
            Ok(Some(answer)) => responses.respond(ClientMessage::String(answer)),
            Ok(None) => responses.respond(ClientMessage::None),
            Err(InquireError::OperationCanceled | InquireError::OperationInterrupted) => {
                responses.respond(ClientMessage::Abort);
            }
            Err(error) => responses.respond(ClientMessage::Error(error.to_string())),
        }
    } else {
        match prompt.prompt() {
            Ok(answer) => responses.respond(ClientMessage::String(answer)),
            Err(InquireError::OperationCanceled | InquireError::OperationInterrupted) => {
                responses.respond(ClientMessage::Abort);
            }
            Err(error) => responses.respond(ClientMessage::Error(error.to_string())),
        }
    }
}
