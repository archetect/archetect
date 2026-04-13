use archetect_api::{ClientMessage, EditorPromptInfo, PromptInfo, PromptInfoLengthRestrictions};
use archetect_validations::validate_text_length;
use inquire::validator::Validation;
use inquire::{Editor, InquireError};

use crate::get_render_config;
use crate::responder::Responder;

pub fn handle_editor_prompt(prompt_info: EditorPromptInfo, responses: &dyn Responder) {
    let help_str = prompt_info.help().map(|v| v.to_string());
    let text = prompt_info.default();
    let is_optional = prompt_info.optional();
    let min = prompt_info.min();
    let max = prompt_info.max();

    // Required: Esc reprompts (opens the editor again), Ctrl+C aborts.
    // Optional: Esc skips (→ None), Ctrl+C aborts.
    loop {
        let mut prompt = Editor::new(prompt_info.message()).with_render_config(get_render_config());
        prompt.predefined_text = text.as_deref();
        prompt.help_message = help_str.as_deref();
        prompt = prompt.with_validator(move |input: &str| match validate_text_length(min, max, input) {
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
                responses.respond(ClientMessage::String(answer));
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
