use archetect_api::{ClientIoHandle, ClientMessage, MultiSelectPromptInfo, PromptInfo, PromptInfoPageable};
use archetect_inquire::{InquireError, MultiSelect};

use crate::get_render_config;

pub fn handle_multiselect_prompt<CIO: ClientIoHandle>(prompt_info: MultiSelectPromptInfo,
                                                      client_handle: CIO) {
    let mut prompt =
        MultiSelect::new(prompt_info.message(), prompt_info.options().to_vec()).with_render_config(get_render_config());

    let mut indices = vec![];
    if let Some(defaults) = prompt_info.defaults() {
        for default in defaults.iter() {
            if let Some(position) = prompt_info
                .options()
                .iter()
                .position(|option| option.to_string().as_str() == default.to_string().as_str())
            {
                indices.push(position);
            }
        }
        prompt = prompt.with_default(&indices);
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
                client_handle
                    .send(ClientMessage::Array(answer));
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
