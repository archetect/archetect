use std::ops::{RangeFrom, RangeInclusive, RangeToInclusive};

use archetect_api::{ClientMessage, ListPromptInfo, PromptInfo, PromptInfoItemsRestrictions};
use inquire::{InquireError, Text};

use crate::get_render_config;
use crate::responder::Responder;

pub fn handle_list_prompt(prompt_info: ListPromptInfo, responses: &dyn Responder) {
    let min_items = prompt_info.min_items();
    let max_items = prompt_info.max_items();
    let help_str = prompt_info.help().map(|v| v.to_string());
    let is_optional = prompt_info.optional();

    let mut items: Vec<String> = Vec::new();

    loop {
        let item_num = items.len() + 1;
        let message = if items.is_empty() {
            format!("{} (empty input when done)", prompt_info.message())
        } else {
            format!("{} [{}] (empty input when done)", prompt_info.message(), item_num)
        };

        let mut prompt = Text::new(&message).with_render_config(get_render_config());
        prompt.help_message = help_str.as_deref();

        // Items always use prompt_skippable so empty input = done adding.
        // Esc behavior: for required lists, re-prompt the current item
        // (don't terminate the list); for optional lists, Esc ends input
        // at whatever's been collected so far. Ctrl+C always aborts.
        match prompt.prompt_skippable() {
            Ok(Some(value)) => {
                if value.is_empty() {
                    break;
                }
                items.push(value);

                if let Some(max) = max_items {
                    if items.len() >= max {
                        break;
                    }
                }
            }
            Ok(None) => {
                // Esc pressed. Optional list: finalize with what we have.
                // Required list: fall through to break and let the size
                // validator decide; if minimum not met, caller sees
                // Error and can retry at the script level. (We can't
                // meaningfully re-prompt for "empty" since Esc IS the
                // finish signal during list entry.)
                if is_optional {
                    break;
                }
                break;
            }
            Err(InquireError::OperationInterrupted) => {
                responses.respond(ClientMessage::Abort);
                return;
            }
            Err(InquireError::OperationCanceled) => {
                // Unexpected in skippable mode, but be defensive: treat
                // the same as Ok(None).
                break;
            }
            Err(error) => {
                responses.respond(ClientMessage::Error(error.to_string()));
                return;
            }
        }
    }

    // Validate list constraints
    if let Err(message) = validate_list(min_items, max_items, &items) {
        responses.respond(ClientMessage::Error(message));
        return;
    }

    responses.respond(ClientMessage::Array(items));
}

fn validate_list(min_items: Option<usize>, max_items: Option<usize>, input: &[String]) -> Result<(), String> {
    let length = input.len();
    match (min_items, max_items) {
        (Some(start), Some(end)) => {
            if !RangeInclusive::new(start, end).contains(&length) {
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
        (None, None) => {}
    }
    Ok(())
}
