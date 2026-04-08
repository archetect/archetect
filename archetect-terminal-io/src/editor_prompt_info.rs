use archetect_api::{ClientMessage, EditorPromptInfo, PromptInfo, PromptInfoLengthRestrictions};
use archetect_validations::validate_text_length;
use inquire::validator::Validation;
use inquire::{Editor, InquireError};

use crate::get_render_config;
use crate::responder::Responder;

pub fn handle_editor_prompt(prompt_info: EditorPromptInfo, responses: &dyn Responder) {
    let help_str = prompt_info.help().map(|v| v.to_string());
    let text = prompt_info.default();

    let mut prompt = Editor::new(prompt_info.message()).with_render_config(get_render_config());
    prompt.predefined_text = text.as_deref();
    prompt.help_message = help_str.as_deref();

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
                responses.respond(ClientMessage::String(answer));
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
