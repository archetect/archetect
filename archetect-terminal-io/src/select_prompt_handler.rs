use log::warn;

use archetect_api::{ClientMessage, PromptInfo, PromptInfoPageable, SelectPromptInfo};
use inquire::{InquireError, Select, Text};

use crate::get_render_config;
use crate::responder::Responder;

pub fn handle_select_prompt(prompt_info: SelectPromptInfo, responses: &dyn Responder) {
    let help_str = prompt_info.help().map(|v| v.to_string());
    let is_optional = prompt_info.optional();
    let allow_other = prompt_info.allow_other();
    let other_label = prompt_info.other_label().to_string();

    // When allow_other is set, append the sentinel entry. We render against
    // an extended option list but answer with the user's typed value (or the
    // selected canonical option) — never the sentinel itself.
    let mut display_options: Vec<String> = prompt_info.options().to_vec();
    if allow_other {
        display_options.push(other_label.clone());
    }

    let mut prompt = Select::new(prompt_info.message(), display_options.clone())
        .with_render_config(get_render_config());

    // Default cursor placement:
    //   - default matches an option → cursor on that option
    //   - default doesn't match AND allow_other → cursor on the "other" entry,
    //     and we'll pre-fill the follow-up text prompt with the default
    //   - default doesn't match AND no allow_other → warn (existing behavior)
    let mut prefill_other: Option<String> = None;
    if let Some(defaults_with) = prompt_info.default() {
        let position = prompt_info
            .options()
            .iter()
            .position(|item| item.as_str() == defaults_with);
        match position {
            Some(idx) => prompt.starting_cursor = idx,
            None if allow_other => {
                prompt.starting_cursor = display_options.len() - 1;
                prefill_other = Some(defaults_with);
            }
            None => warn!("A 'defaults_with' was set, but did not match any of the options."),
        }
    }

    prompt.help_message = help_str.as_deref();

    if let Some(page_size) = prompt_info.page_size() {
        prompt.page_size = page_size;
    }

    let select_result = if is_optional {
        match prompt.prompt_skippable() {
            Ok(Some(answer)) => Ok(Some(answer)),
            Ok(None) => Ok(None),
            Err(e) => Err(e),
        }
    } else {
        prompt.prompt().map(Some)
    };

    match select_result {
        Ok(Some(answer)) if allow_other && answer == other_label => {
            // User picked the sentinel — collect the freeform value.
            let mut text = Text::new(prompt_info.message()).with_render_config(get_render_config());
            text.help_message = help_str.as_deref();
            if let Some(ref pre) = prefill_other {
                text.initial_value = Some(pre);
            }
            let text_result = if is_optional {
                text.prompt_skippable().map(|opt| opt.filter(|s| !s.is_empty()))
            } else {
                text.prompt().map(Some)
            };
            match text_result {
                Ok(Some(value)) => responses.respond(ClientMessage::String(value)),
                Ok(None) => responses.respond(ClientMessage::None),
                Err(InquireError::OperationCanceled | InquireError::OperationInterrupted) => {
                    responses.respond(ClientMessage::Abort);
                }
                Err(error) => responses.respond(ClientMessage::Error(error.to_string())),
            }
        }
        Ok(Some(answer)) => responses.respond(ClientMessage::String(answer)),
        Ok(None) => responses.respond(ClientMessage::None),
        Err(InquireError::OperationCanceled | InquireError::OperationInterrupted) => {
            responses.respond(ClientMessage::Abort);
        }
        Err(error) => responses.respond(ClientMessage::Error(error.to_string())),
    }
}
