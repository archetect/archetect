use linked_hash_map::LinkedHashMap;
use read_input::prelude::*;

use crate::{Archetect, ArchetectError};
use crate::config::{AnswerInfo, VariableInfo};
use crate::template_engine::Context;

pub fn populate_context(
    archetect: &Archetect,
    variables: &LinkedHashMap<String, VariableInfo>,
    answers: &LinkedHashMap<String, AnswerInfo>,
    context: &mut Context
) -> Result<(), ArchetectError> {

    for (identifier, variable_info) in variables {
        // 1) If there is an answer for this variable, and has an explicit value, use that first.
        if let Some(answer) = answers.get(identifier) {
            if let Some(value) = answer.value() {
                context.insert(
                    identifier.as_str(),
                    &archetect.render_string(value, context)?
                );
                continue;
            }
        }

        let mut prompt = if let Some(prompt) = variable_info.prompt() {
            format!("{} ", archetect.render_string(prompt.trim(), context)?)
        } else {
            format!("{}: ", identifier)
        };

        // Insert a value if one was specified in the archetype's configuration file.
        if let Some(value) = variable_info.value() {
            context.insert(
                identifier.as_str(),
                &archetect.render_string(value, context)?,
            );
            continue;
        }

        // Determine if a default can be provided.
        let default = if let Some(answer) = answers.get(identifier) {
            if let Some(default) = answer.default() {
                Some(archetect.render_string(default, context)?)
            } else {
                None
            }
        } else if let Some(default) = variable_info.default() {
            Some(archetect.render_string(default, context)?)
        } else {
            None
        };

        if let Some(default) = &default {
            prompt.push_str(format!("[{}] ", default).as_str());
        };

        let input_builder = input::<String>()
            .msg(&prompt)
            .add_test(|value| value.len() > 0)
            .repeat_msg(&prompt)
            .err("Must be at least 1 character.  Please try again.");
        let value = if let Some(default) = &default {
            input_builder.default(default.clone().to_owned()).get()
        } else {
            input_builder.get()
        };

        context.insert(identifier, &value);
    }

    Ok(())
}

pub fn render_answers(
    archetect: &Archetect,
    answers: &LinkedHashMap<String, AnswerInfo>,
    context: &Context
) -> Result<LinkedHashMap<String, AnswerInfo>, ArchetectError> {
    let mut results = LinkedHashMap::new();
    for (identifier, answer_info) in answers {
        let mut result = AnswerInfo::new();
        if let Some(value) = answer_info.value() {
            result = result.with_value(archetect.render_string(value, context)?);
        }
        if let Some(prompt) = answer_info.prompt() {
            result = result.with_prompt(archetect.render_string(prompt, context)?);
        }
        if let Some(default) = answer_info.default() {
            result = result.with_default(archetect.render_string(default, context)?);
        }
        if answer_info.is_inheritable() {
            result = result.inheritable();
        }
        results.insert(identifier.to_owned(), result.build());
    }
    Ok(results)
}