use archetect_api::{ClientIoHandle, ClientMessage, EditorPromptInfo, PromptInfo, PromptInfoLengthRestrictions};
use archetect_inquire::{Editor, InquireError};
use archetect_inquire::validator::Validation;
use archetect_validations::validate_text_length;

use crate::get_render_config;

pub fn handle_editor_prompt<CIO: ClientIoHandle>(prompt_info: EditorPromptInfo, client_handle: CIO) {
    let mut prompt = Editor::new(prompt_info.message()).with_render_config(get_render_config());
    let text = prompt_info.default();
    prompt.predefined_text = text.as_deref();
    if prompt_info.help().is_some() {
        prompt.help_message = prompt_info.help();
    }

    let min = prompt_info.min();
    let max = prompt_info.max();
    let validator = move |input: &str| {
        match validate_text_length(min, max, input) {
            Ok(_) => Ok(Validation::Valid),
            Err(message) => Ok(Validation::Invalid(message.into())),
        }
    };
    prompt = prompt.with_validator(validator);
    match prompt.prompt_skippable() {
        Ok(answer) => {
            if let Some(answer) = answer {
                client_handle
                    .send(ClientMessage::String(answer));
            } else {
                client_handle.send(ClientMessage::None);
            }
        }
        Err(error) => {
            match error {
                InquireError::OperationCanceled | InquireError::OperationInterrupted => {
                    client_handle.send(ClientMessage::Abort);
                }
                _ => {
                    client_handle
                        .send(ClientMessage::Error(error.to_string()));
                }
            }
        }
    }
}
