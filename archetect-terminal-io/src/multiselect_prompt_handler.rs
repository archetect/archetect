use archetect_api::{ClientMessage, MultiSelectPromptInfo, PromptInfo, PromptInfoPageable};
use inquire::{InquireError, MultiSelect};

use crate::get_render_config;
use crate::responder::Responder;

pub fn handle_multiselect_prompt(prompt_info: MultiSelectPromptInfo, responses: &dyn Responder) {
    let help_str = prompt_info.help().map(|v| v.to_string());

    let mut prompt =
        MultiSelect::new(prompt_info.message(), prompt_info.options().to_vec()).with_render_config(get_render_config());

    let mut indices = vec![];
    if let Some(defaults) = prompt_info.defaults() {
        for default in defaults.iter() {
            if let Some(position) = prompt_info
                .options()
                .iter()
                .position(|option| option.as_str() == default.as_str())
            {
                indices.push(position);
            }
        }
        prompt = prompt.with_default(&indices);
    }

    prompt.help_message = help_str.as_deref();

    if let Some(page_size) = prompt_info.page_size() {
        prompt.page_size = page_size;
    }

    match prompt.prompt_skippable() {
        Ok(answer) => {
            if let Some(answer) = answer {
                responses.respond(ClientMessage::Array(answer));
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
