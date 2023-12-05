use std::sync::mpsc::SyncSender;

use log::warn;

use archetect_api::{CommandResponse, PromptInfo, SelectPromptInfo};
use archetect_inquire::Select;

use crate::get_render_config;

pub fn handle_select_prompt(prompt_info: SelectPromptInfo, responses: &SyncSender<CommandResponse>) {
    let mut prompt =
        Select::new(prompt_info.message(), prompt_info.options().to_vec()).with_render_config(get_render_config());

    if let Some(defaults_with) = prompt_info.default() {
        let default = prompt_info
            .options()
            .iter()
            .position(|item| item.to_string().as_str() == defaults_with.to_string().as_str());
        if let Some(default) = default {
            prompt.starting_cursor = default;
        } else {
            warn!("A 'defaults_with' was set, but did not match any of the options.");
        }
    }

    prompt.help_message = prompt_info.help().map(|v| v.to_string());

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
