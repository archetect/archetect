use archetect_api::{ClientIoHandle, ClientMessage, PromptInfo, PromptInfoPageable, SelectPromptInfo};
use archetect_inquire::{InquireError, Select};
use tracing::warn;

use crate::get_render_config;

pub fn handle_select_prompt<CIO: ClientIoHandle>(prompt_info: SelectPromptInfo, client_handle: &CIO) {
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

    if prompt_info.help().is_some() {
        prompt.help_message = prompt_info.help().map(|v| v.to_string());
    }

    if let Some(page_size) = prompt_info.page_size() {
        prompt.page_size = page_size;
    }

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
