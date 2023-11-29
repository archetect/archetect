use std::ops::{RangeFrom, RangeInclusive, RangeToInclusive};
use std::sync::mpsc::SyncSender;
use inquire::Text;
use inquire::validator::Validation;
use archetect_api::{CommandResponse, IntPromptInfo, PromptInfo};
use crate::get_render_config;

pub fn handle_prompt_int(prompt_info: IntPromptInfo, responses: &SyncSender<CommandResponse>) {
    let mut prompt = Text::new(prompt_info.message())
        .with_render_config(get_render_config())
        ;
    let default = prompt_info.default().map(|v| v.to_string());
    prompt.default = default;
    prompt.placeholder = prompt_info.placeholder().map(|v|v.to_string());
    prompt.help_message = prompt_info.help().map(|v|v.to_string());
    let prompt_info = prompt_info.clone();
    let validator = move |input: &str| match validate_int(prompt_info.min(), prompt_info.max(), input) {
        Ok(_) => Ok(Validation::Valid),
        Err(message) => Ok(Validation::Invalid(message.into())),
    };
    prompt = prompt.with_validator(validator);
    match prompt.prompt_skippable() {
        Ok(answer) => {
            if let Some(answer) = answer {
                responses
                    .send(CommandResponse::IntAnswer(
                        answer.parse::<i64>().expect("Pre-validated"),
                    ))
                    .expect("Channel Send Error");
            } else {
                responses.send(CommandResponse::NoneAnswer).expect("Channel Send Error");
            }
        }
        Err(error) => {
            responses
                .send(CommandResponse::Error(error.to_string()))
                .expect("Channel Send Error");
        }
    }
}


fn validate_int(min: Option<i64>, max: Option<i64>, input: &str) -> Result<(), String> {
    match input.parse::<i64>() {
        Ok(value) => {
            match (min, max) {
                (Some(start), Some(end)) => {
                    if !RangeInclusive::new(start, end).contains(&value) {
                        return Err(format!("Answer must be between {} and {}", start, end));
                    }
                }
                (Some(start), None) => {
                    if !(RangeFrom { start }.contains(&value)) {
                        return Err(format!("Answer must be greater than {}", start));
                    }
                }
                (None, Some(end)) => {
                    if !(RangeToInclusive { end }.contains(&value)) {
                        return Err(format!("Answer must be less than or equal to {}", end));
                    }
                }
                (None, None) => {}
            };

            Ok(())
        }
        Err(_) => Err(format!("{} is not an 'int'", input)),
    }
}