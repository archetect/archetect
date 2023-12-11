use std::sync::mpsc::SyncSender;

use archetect_api::{CommandResponse, PromptInfo, PromptInfoLengthRestrictions, TextPromptInfo};
use archetect_validations::validate_text_length;
use archetect_inquire::Text;
use archetect_inquire::validator::Validation;

use crate::get_render_config;

pub fn handle_prompt_text(prompt_info: TextPromptInfo, responses: &SyncSender<CommandResponse>) {
    let mut prompt = Text::new(prompt_info.message()).with_render_config(get_render_config());
    prompt.default = prompt_info.default().map(|v| v.to_string());
    prompt.placeholder = prompt_info.placeholder().map(|v| v.to_string());
    if prompt_info.help().is_some() {
        prompt.help_message = prompt_info.help().map(|v| v.to_string());
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
