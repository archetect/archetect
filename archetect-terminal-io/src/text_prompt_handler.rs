use archetect_api::{ClientIoHandle, ClientMessage, PromptInfo, PromptInfoLengthRestrictions, TextPromptInfo};
use archetect_inquire::{InquireError, Text};
use archetect_inquire::validator::Validation;
use archetect_validations::validate_text_length;

use crate::get_render_config;

pub fn handle_prompt_text<CIO: ClientIoHandle>(prompt_info: TextPromptInfo, client_handle: &CIO) {
    let mut prompt = Text::new(prompt_info.message()).with_render_config(get_render_config());
    prompt.default = prompt_info.default().map(|v| v.to_string());
    prompt.placeholder = prompt_info.placeholder().map(|v| v.to_string());
    if prompt_info.help().is_some() {
        prompt.help_message = prompt_info.help().map(|v| v.to_string());
    }
    let min = prompt_info.min();
    let max = prompt_info.max();
    let validator = move |input: &str| match validate_text_length(min, max, input) {
        Ok(_) => Ok(Validation::Valid),
        Err(message) => Ok(Validation::Invalid(message.into())),
    };
    prompt = prompt.with_validator(validator);
    match prompt.prompt_skippable() {
        Ok(answer) => {
            if let Some(answer) = answer {
                client_handle.send(ClientMessage::String(answer));
            } else {
                client_handle.send(ClientMessage::None);
            }
        }
        Err(error) => match error {
            InquireError::OperationCanceled | InquireError::OperationInterrupted => {
                client_handle.send(ClientMessage::Abort);
            }
            _ => {
                client_handle.send(ClientMessage::Error(error.to_string()));
            }
        },
    }
}
