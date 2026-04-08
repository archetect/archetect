use log::warn;

use archetect_api::{ClientMessage, PromptInfo, PromptInfoPageable, SelectPromptInfo};
use inquire::{InquireError, Select};

use crate::get_render_config;
use crate::responder::Responder;

pub fn handle_select_prompt(prompt_info: SelectPromptInfo, responses: &dyn Responder) {
    let help_str = prompt_info.help().map(|v| v.to_string());

    let mut prompt =
        Select::new(prompt_info.message(), prompt_info.options().to_vec()).with_render_config(get_render_config());

    if let Some(defaults_with) = prompt_info.default() {
        let default = prompt_info
            .options()
            .iter()
            .position(|item| item.as_str() == defaults_with);
        if let Some(default) = default {
            prompt.starting_cursor = default;
        } else {
            warn!("A 'defaults_with' was set, but did not match any of the options.");
        }
    }

    prompt.help_message = help_str.as_deref();

    if let Some(page_size) = prompt_info.page_size() {
        prompt.page_size = page_size;
    }

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
