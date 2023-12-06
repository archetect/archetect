use std::sync::mpsc::SyncSender;

use archetect_api::{CommandResponse, PromptInfo, PromptInfoLengthRestrictions, EditorPromptInfo};
use archetect_api::validations::{validate_text};
use archetect_inquire::Editor;
use archetect_inquire::validator::Validation;

use crate::get_render_config;

pub fn handle_editor_prompt(prompt_info: EditorPromptInfo, responses: &SyncSender<CommandResponse>) {
    let mut prompt = Editor::new(prompt_info.message()).with_render_config(get_render_config());
    let text = prompt_info.default();
    prompt.predefined_text = text.as_deref();
    prompt.help_message = prompt_info.help();
    let min = prompt_info.min();
    let max = prompt_info.max();
    let validator = move |input: &str| {
        match validate_text(min, max, input) {
            Ok(_) => Ok(Validation::Valid),
            Err(message) => Ok(Validation::Invalid(message.into())),
        }
    };
    prompt = prompt.with_validator(validator);
    match prompt.prompt_skippable() {
        Ok(answer) => {
            if let Some(answer) = answer {
                responses
                    .send(CommandResponse::String(answer))
                    .expect("Channel Send Error");
            } else {
                responses.send(CommandResponse::None).expect("Channel Send Error");
            }
        }
        Err(error) => {
            responses
                .send(CommandResponse::Error(error.to_string()))
                .expect("Channel Send Error");
        }
    }
}
