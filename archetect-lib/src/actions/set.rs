use linked_hash_map::LinkedHashMap;
use read_input::prelude::*;

use crate::{Archetect, ArchetectError};
use crate::config::{AnswerInfo, VariableInfo, VariableType};
use crate::template_engine::Context;
use serde_json::Value;
use std::collections::{HashMap, HashSet};

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
                match variable_info.variable_type() {
                    VariableType::Enum(options) => {
                        if options.contains(&value.to_owned()) {
                            context.insert(
                                identifier.as_str(),
                                &archetect.render_string(value, context)?
                            );
                            continue;
                        }
                    }
                    _ => {
                        context.insert(
                            identifier.as_str(),
                            &archetect.render_string(value, context)?
                        );
                        continue;
                    }
                }
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
            } else if let Some(default) = variable_info.default() {
                Some(archetect.render_string(default, context)?)
            } else {
                None
            }
        } else if let Some(default) = variable_info.default() {
            Some(archetect.render_string(default, context)?)
        } else {
            None
        };

        let value = match variable_info.variable_type() {
            VariableType::Enum(values) => {
                prompt_for_enum(&mut prompt, &values, &default)
            }
            _ => { prompt_for_string(&mut prompt, &default, variable_info.required())}
        };

        if let Some(value) = value {
            context.insert(identifier, &value);
        }
    }

    Ok(())
}

fn prompt_for_string(prompt: &mut String, default: &Option<String>, required: bool) ->  Option<Value> {
    if let Some(default) = &default {
        prompt.push_str(format!("[{}] ", default).as_str());
    };
    let mut input_builder = input::<String>().msg(&prompt);

    if required {
        input_builder = input_builder.add_test(|value| value.len() > 0)
        .repeat_msg(&prompt)
        .err("Must be at least 1 character.  Please try again.");
    }

    let value = if let Some(default) = &default {
        input_builder.default(default.clone().to_owned()).get()
    } else {
        input_builder.get()
    };
    Some(Value::String(value))
}

fn prompt_for_enum(prompt: &mut String, options: &Vec<String>, default: &Option<String>) ->  Option<Value> {
    println!("{}", &prompt);
    let choices = options
        .iter()
        .enumerate()
        .map(|(id, entry)| (id + 1, entry.clone()))
        .collect::<HashMap<_, _>>();

    for (id, option) in options.iter().enumerate() {
        println!("{:>2}) {}", id + 1, option);
    }

    let mut message = String::from("Select and entry: ");
    if let Some(default) = default {
        if options.contains(default) {
            message.push_str(format!("[{}] ", default).as_str());
        } else {
            use log::info;
            info!("{} not found in {:?}", default, options);
        }
    };

    let test_values = choices.keys().map(|v| *v).collect::<HashSet<_>>();

    let input_builder = input::<usize>()
        .msg(&message)
        .add_test(move |value| test_values.contains(value))
        .err("Please enter the number of a selection from the list.")
        .repeat_msg(&message);

    let value = if let Some(default) = default {
        input_builder.default(options.iter().position(|e| e.eq(default)).unwrap() + 1).get()
    } else {
        input_builder.get()
    };

    Some(Value::String(choices.get(&value).unwrap().to_owned()))
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
        results.insert(identifier.to_owned(), result.build());
    }
    Ok(results)
}