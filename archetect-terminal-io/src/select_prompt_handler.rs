use log::warn;

use archetect_api::{ClientMessage, PromptInfo, PromptInfoPageable, SelectPromptInfo};
use inquire::validator::Validation;
use inquire::{InquireError, Select, Text};

use crate::get_render_config;
use crate::responder::Responder;

pub fn handle_select_prompt(prompt_info: SelectPromptInfo, responses: &dyn Responder) {
    let help_str = prompt_info.help().map(|v| v.to_string());
    let is_optional = prompt_info.optional();
    let allow_other = prompt_info.allow_other();
    let other_label = prompt_info.other_label().to_string();

    let mut display_options: Vec<String> = prompt_info.options().to_vec();
    if allow_other {
        display_options.push(other_label.clone());
    }

    // Compute default cursor placement once; rebuild the Select each loop
    // iteration so inquire can redraw cleanly when we reprompt on Esc.
    let default_cursor = prompt_info
        .default()
        .map(|d| {
            let in_opts = prompt_info
                .options()
                .iter()
                .position(|item| item.as_str() == d);
            match (in_opts, allow_other) {
                (Some(idx), _) => Some((idx, None)),
                (None, true) => Some((display_options.len() - 1, Some(d))),
                (None, false) => {
                    warn!("A 'defaults_with' was set, but did not match any of the options.");
                    None
                }
            }
        })
        .flatten();

    let page_size = prompt_info.page_size();

    // Required prompts: Esc reprompts, Ctrl+C aborts. Optional prompts:
    // Esc skips (→ None), Ctrl+C aborts. Loop only for the required path.
    let raw_answer = loop {
        let mut select =
            Select::new(prompt_info.message(), display_options.clone()).with_render_config(get_render_config());
        select.help_message = help_str.as_deref();
        if let Some(p) = page_size {
            select.page_size = p;
        }
        if let Some((idx, _)) = default_cursor.as_ref() {
            select.starting_cursor = *idx;
        }

        let result = if is_optional {
            select.prompt_skippable()
        } else {
            select.prompt().map(Some)
        };

        match result {
            Ok(Some(v)) => break Some(v),
            Ok(None) => break None,
            Err(InquireError::OperationCanceled) if !is_optional => continue,
            Err(InquireError::OperationCanceled | InquireError::OperationInterrupted) => {
                if matches!(
                    result,
                    Err(InquireError::OperationInterrupted)
                ) {
                    responses.respond(ClientMessage::Abort);
                } else {
                    responses.respond(ClientMessage::None);
                }
                return;
            }
            Err(error) => {
                responses.respond(ClientMessage::Error(error.to_string()));
                return;
            }
        }
    };

    let answer = match raw_answer {
        Some(a) => a,
        None => {
            responses.respond(ClientMessage::None);
            return;
        }
    };

    // User picked the "Other..." sentinel — collect freeform text.
    if allow_other && answer == other_label {
        let prefill = default_cursor.as_ref().and_then(|(_, pre)| pre.clone());
        collect_other_text(prompt_info.message(), help_str.as_deref(), is_optional, prefill, responses);
        return;
    }

    responses.respond(ClientMessage::String(answer));
}

fn collect_other_text(
    message: &str,
    help: Option<&str>,
    is_optional: bool,
    prefill: Option<String>,
    responses: &dyn Responder,
) {
    loop {
        let mut text = Text::new(message).with_render_config(get_render_config());
        text.help_message = help;
        if let Some(ref pre) = prefill {
            text.initial_value = Some(pre);
        }
        if !is_optional {
            text = text.with_validator(|input: &str| {
                if input.is_empty() {
                    Ok(Validation::Invalid("Answer is required.".into()))
                } else {
                    Ok(Validation::Valid)
                }
            });
        }

        let result = if is_optional {
            text.prompt_skippable().map(|opt| opt.filter(|s| !s.is_empty()))
        } else {
            text.prompt().map(Some)
        };

        match result {
            Ok(Some(v)) => {
                responses.respond(ClientMessage::String(v));
                return;
            }
            Ok(None) => {
                responses.respond(ClientMessage::None);
                return;
            }
            Err(InquireError::OperationCanceled) if !is_optional => continue,
            Err(InquireError::OperationInterrupted) => {
                responses.respond(ClientMessage::Abort);
                return;
            }
            Err(InquireError::OperationCanceled) => {
                responses.respond(ClientMessage::None);
                return;
            }
            Err(error) => {
                responses.respond(ClientMessage::Error(error.to_string()));
                return;
            }
        }
    }
}
