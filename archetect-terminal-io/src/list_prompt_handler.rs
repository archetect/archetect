use std::ops::{RangeFrom, RangeInclusive, RangeToInclusive};
use std::sync::mpsc::SyncSender;

use archetect_api::{CommandResponse, ListPromptInfo, PromptInfo};
use inquire::List;
use inquire::validator::Validation;

use crate::get_render_config;

pub fn handle_list_prompt(prompt_info: ListPromptInfo, responses: &SyncSender<CommandResponse>) {
    let min_items = prompt_info.min_items();
    let max_items = prompt_info.max_items();
    let list_validator = move |input: &Vec<String>| match validate_list(min_items, max_items, input) {
        Ok(_) => return Ok(Validation::Valid),
        Err(message) => return Ok(Validation::Invalid(message.into())),
    };
    let mut prompt = List::new(prompt_info.message())
        .with_list_validator(list_validator)
        .with_render_config(get_render_config())
        ;

    prompt.defaults = prompt_info.defaults();
    prompt.placeholder = prompt_info.placeholder().map(|v|v.to_string());
    prompt.help_message = prompt_info.help().map(|v|v.to_string());

    match prompt.prompt_skippable() {
        Ok(answer) => {
            if let Some(answer) = answer {
                responses
                    .send(CommandResponse::MultiStringAnswer(answer))
                    .expect("Channel Send Error");
            } else {
                responses.send(CommandResponse::NoneAnswer)
                    .expect("Channel Send Error");
            }
        }
        Err(error) => {
            responses
                .send(CommandResponse::Error(error.to_string()))
                .expect("Channel Send Error");
        }
    }
}

fn validate_list(min_items: Option<usize>, max_items: Option<usize>, input: &Vec<String>) -> Result<(), String> {
    let length = input.len();
    match (min_items, max_items) {
        (Some(start), Some(end)) => {
            if !RangeInclusive::new(start, end).contains(&input.len()) {
                return Err(format!("List must have between {} and {} items", start, end));
            }
        }
        (Some(start), None) => {
            if !(RangeFrom { start }.contains(&length)) {
                return Err(format!("List must have at least {} items", start));
            }
        }
        (None, Some(end)) => {
            if !(RangeToInclusive { end }.contains(&length)) {
                return Err(format!("List must have no more than {} items", end));
            }
        }
        (None, None) => return Ok(()),
    };

    Ok(())
}
