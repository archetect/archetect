use std::sync::mpsc::SyncSender;

use archetect_api::{CommandResponse, MultiSelectPromptInfo, PromptInfo, PromptInfoPageable};
use archetect_inquire::MultiSelect;

use crate::get_render_config;

pub fn handle_multiselect_prompt(prompt_info: MultiSelectPromptInfo, responses: &SyncSender<CommandResponse>) {
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

    prompt.help_message = prompt_info.help().map(|v| v.to_string());

    if let Some(page_size) = prompt_info.page_size() {
        prompt.page_size = page_size;
    }

    match prompt.prompt_skippable() {
        Ok(answer) => {
            if let Some(answer) = answer {
                responses
                    .send(CommandResponse::Array(answer))
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
