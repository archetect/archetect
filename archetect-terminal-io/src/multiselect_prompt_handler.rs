use archetect_api::{ClientMessage, MultiSelectPromptInfo, PromptInfo, PromptInfoPageable};
use inquire::{InquireError, MultiSelect};

use crate::get_render_config;
use crate::responder::Responder;

pub fn handle_multiselect_prompt(prompt_info: MultiSelectPromptInfo, responses: &dyn Responder) {
    let help_str = prompt_info.help().map(|v| v.to_string());
    let is_optional = prompt_info.optional();
    let page_size = prompt_info.page_size();

    // Precompute default indices once.
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
    }

    // Required: Esc reprompts, Ctrl+C aborts. Optional: Esc skips, Ctrl+C aborts.
    loop {
        let mut prompt = MultiSelect::new(prompt_info.message(), prompt_info.options().to_vec())
            .with_render_config(get_render_config());
        prompt.help_message = help_str.as_deref();
        if let Some(p) = page_size {
            prompt.page_size = p;
        }
        if !indices.is_empty() {
            prompt = prompt.with_default(&indices);
        }

        let result = if is_optional {
            prompt.prompt_skippable()
        } else {
            prompt.prompt().map(Some)
        };

        match result {
            Ok(Some(answer)) => {
                responses.respond(ClientMessage::Array(answer));
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
