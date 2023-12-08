use std::sync::mpsc::SyncSender;

use archetect_inquire::Confirm;

use archetect_api::{BoolPromptInfo, CommandResponse, ValueSource};

use crate::get_render_config;

pub fn handle_prompt_bool(prompt_info: BoolPromptInfo, responses: &SyncSender<CommandResponse>) {
    let mut prompt = Confirm::new(prompt_info.message()).with_render_config(get_render_config());
    let default = prompt_info.default();
    prompt.default = default;
    prompt.placeholder = prompt_info.placeholder().map(|v| v.to_string());
    if prompt_info.help().is_some() {
        prompt.help_message = prompt_info.help().map(|v| v.to_string());
    }
    let prompt_info = prompt_info.clone();
    let parser = |ans: &str| {
        if ans.len() > 5 {
            return Err(());
        }
        get_boolean::<&str>(ans, prompt_info.message(), None, ValueSource::Value).map_err(|_| ())
    };
    prompt.parser = &parser;
    match prompt.prompt_skippable() {
        Ok(answer) => {
            if let Some(answer) = answer {
                responses
                    .send(CommandResponse::Boolean(answer))
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

fn get_boolean<K: AsRef<str>>(value: &str, prompt: &str, key: Option<K>, source: ValueSource) -> Result<bool, String> {
    match value.to_lowercase().as_str() {
        "y" | "yes" | "t" | "true" => Ok(true),
        "n" | "no" | "f" | "false" => Ok(false),
        _ => Err(match key {
            Some(key) => {
                format!(
                    "'{}' was provided as {} to prompt: '{}' with Key: '{}', but must resemble a boolean",
                    value.to_string(),
                    source.description(),
                    prompt,
                    key.as_ref(),
                )
            }
            None => {
                format!(
                    "'{}' was provided as {} to prompt: '{}', but must resemble a boolean",
                    value.to_string(),
                    source.description(),
                    prompt,
                )
            }
        }),
    }
}
