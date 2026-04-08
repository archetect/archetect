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

    let mut prompt = Text::new(prompt_info.message()).with_render_config(get_render_config());
    prompt.default = default_str.as_deref();
    prompt.placeholder = placeholder_str.as_deref();
    prompt.help_message = help_str.as_deref();

    let prompt_info = prompt_info.clone();
    let validator = move |input: &str| match validate(prompt_info.min(), prompt_info.max(), input) {
        Ok(_) => Ok(Validation::Valid),
        Err(message) => Ok(Validation::Invalid(message.into())),
    };
    prompt = prompt.with_validator(validator);
    match prompt.prompt_skippable() {
        Ok(answer) => {
            if let Some(answer) = answer {
                responses.respond(ClientMessage::Integer(answer.parse::<i64>().expect("Pre-validated")));
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

fn validate(min: Option<i64>, max: Option<i64>, input: &str) -> Result<(), String> {
    match input.parse::<i64>() {
        Ok(value) => validate_int_size(min, max, value),
        Err(_) => Err(format!("{} is not an 'int'", input)),
    }
}
