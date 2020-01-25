use std::collections::{HashMap, HashSet};

use linked_hash_map::LinkedHashMap;
use read_input::prelude::*;
use serde_json::Value;

use crate::{Archetect, ArchetectError};
use crate::config::{AnswerInfo, VariableInfo, VariableType};
use crate::template_engine::Context;

const ACCEPTABLE_BOOLEANS: [&str; 8] = ["y", "yes", "true", "t", "n", "no", "false", "f"];

pub fn populate_context(
    archetect: &Archetect,
    variables: &LinkedHashMap<String, VariableInfo>,
    answers: &LinkedHashMap<String, AnswerInfo>,
    context: &mut Context,
) -> Result<(), ArchetectError> {
    for (identifier, variable_info) in variables {

        // 1) If there is an answer for this variable, and has an explicit value, use that first.
        if let Some(answer) = answers.get(identifier) {
            let mut answer_satisfied = false;

            if let Some(value) = answer.value() {
                match variable_info.variable_type() {
                    VariableType::Enum(options) => {
                        if options.contains(&value.to_owned()) {
                            context.insert(
                                identifier.as_str(),
                                &archetect.render_string(value, context)?,
                            );
                            answer_satisfied = true;
                        }
                    }
                    VariableType::Bool => {
                        let value = value.to_lowercase();
                        if ACCEPTABLE_BOOLEANS.contains(&value.as_str()) {
                            let value = match ACCEPTABLE_BOOLEANS.iter().position(|i| i == &value.as_str()).unwrap() {
                                0..=3 => true,
                                _ => false,
                            };
                            context.insert(
                                identifier.as_str(),
                                &value,
                            );
                            answer_satisfied = true;
                        }
                    }
                    VariableType::Int => {
                        if let Ok(value) = value.parse::<i64>() {
                            context.insert(
                                identifier.as_str(),
                                &value,
                            );
                            answer_satisfied = true;
                        }
                    }
                    VariableType::String => {
                        context.insert(
                            identifier.as_str(),
                            &archetect.render_string(value, context)?,
                        );
                        answer_satisfied = true;
                    }
                    VariableType::List => {
                        if let Some(variable_value) = variable_info.value() {
                            let mut temp_context = context.clone();
                            temp_context.insert("item", value);
                            context.insert(
                                identifier.as_str(),
                                &archetect.render_string(variable_value, &temp_context)?,
                            );
                            answer_satisfied = true;
                        } else {
                            context.insert(
                                identifier.as_str(),
                                &archetect.render_string(value, context)?,
                            );
                        }
                    }
                }
            }

            if answer_satisfied {
                // Allow answered variables to be formatted or derived
                if let Some(value) = variable_info.value() {
                    match variable_info.variable_type() {
                        // Special handling for lists
                        VariableType::List => {}
                        _ => {
                            context.insert(
                                identifier.as_str(),
                                &archetect.render_string(value, context)?,
                            );
                        }
                    }
                }
                continue;
            }
        }

        // Insert wholly derived values
        if variable_info.has_derived_value() {
            if let Some(value) = variable_info.value() {
                context.insert(
                    identifier.as_str(),
                    &archetect.render_string(value, context)?,
                );
                continue;
            }
        }


        let mut prompt = if let Some(prompt) = variable_info.prompt() {
            format!("{} ", archetect.render_string(prompt.trim(), context)?)
        } else {
            format!("{}: ", identifier)
        };

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
            VariableType::Bool => {
                prompt_for_bool(&mut prompt, &default)
            }
            VariableType::Int => {
                prompt_for_int(&mut prompt, &default)
            }
            VariableType::List => {
                prompt_for_list(archetect, context, &prompt, variable_info)?
            }
            VariableType::String => {
                prompt_for_string(&mut prompt, &default, variable_info.required())
            }
        };

        if let Some(value) = value {
            context.insert(identifier, &value);

            // Allow prompted variables to be formatted or derived
            if let Some(value) = variable_info.value() {
                match variable_info.variable_type() {
                    VariableType::List => (),
                    _ => {
                        context.insert(
                            identifier.as_str(),
                            &archetect.render_string(value, context)?,
                        );
                    }
                }
            }
        }
    }

    Ok(())
}

fn prompt_for_string(prompt: &mut String, default: &Option<String>, required: bool) -> Option<Value> {
    if let Some(default) = &default {
        prompt.push_str(format!("[{}] ", default).as_str());
    };
    let mut input_builder = input::<String>().msg(&prompt);

    if required {
        input_builder = input_builder.add_test(|value| value.len() > 0)
            .repeat_msg(&prompt)
            .err("Please provide a value.");
    }

    let value = if let Some(default) = &default {
        input_builder.default(default.clone().to_owned()).get()
    } else {
        input_builder.get()
    };
    Some(Value::String(value))
}

fn prompt_for_int(prompt: &mut String, default: &Option<String>) -> Option<Value> {
    let default = default.as_ref().map_or(None, |value| value.parse::<i64>().ok());

    if let Some(default) = default {
        prompt.push_str(format!("[{}] ", default).as_str());
    }

    let input_builder = input::<i64>()
        .msg(&prompt)
        .err("Please specify an integer.")
        .repeat_msg(&prompt)
        ;

    let value = if let Some(default) = default {
        input_builder.default(default).get()
    } else {
        input_builder.get()
    };

    Some(Value::from(value))
}

fn prompt_for_bool(prompt: &mut String, default: &Option<String>) -> Option<Value> {
    let default = default.as_ref().map_or(None, |value| {
        let value = value.to_lowercase();
        if ACCEPTABLE_BOOLEANS.contains(&value.as_str()) {
            Some(value.to_owned())
        } else {
            None
        }
    });

    if let Some(default) = default.clone() {
        prompt.push_str(format!("[{}] ", default).as_str());
    }

    let input_builder = input::<String>()
        .add_test(|value| {
            match value.to_lowercase().as_str() {
                "y" | "yes" | "t" | "true" | "n" | "no" | "f" | "false" => true,
                _ => false
            }
        })
        .msg(&prompt)
        .err(format!("Please specify a value of {:?}.", ACCEPTABLE_BOOLEANS))
        .repeat_msg(&prompt)
        ;

    let value = if let Some(default) = default.clone() {
        input_builder.default(default.to_owned()).get()
    } else {
        input_builder.get()
    };

    let value = match ACCEPTABLE_BOOLEANS.iter().position(|i| i == &value.as_str()).unwrap() {
        0..=3 => true,
        _ => false,
    };

    Some(Value::Bool(value))
}

fn prompt_for_list(
    archetect: &Archetect,
    context: &Context,
    prompt: &String,
    variable_info: &VariableInfo,
) -> Result<Option<Value>, ArchetectError> {
    println!("{}", &prompt);

    let mut results = vec![];


    loop {
        let count = results.len();
        let mut input_builder = input::<String>()
            .msg("Item: ")
            ;

        if variable_info.required() {
            input_builder = input_builder.add_test(move |value| count > 0 || !value.trim().is_empty())
                .err("This list requires at least one item.")
                .repeat_msg("Item: ")
        }
        let mut item = input_builder.get();

        if item.trim().is_empty() {
            break;
        }

        if let Some(value) = variable_info.value() {
            let mut context = context.clone();
            context.insert("item", &item);
            item = archetect.render_string(value, &context)?;
        }

        results.push(Value::String(item));
    }

    Ok(Some(Value::Array(results)))
}

fn prompt_for_enum(prompt: &mut String, options: &Vec<String>, default: &Option<String>) -> Option<Value> {
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
        }
    };

    let test_values = choices.keys().map(|v| *v).collect::<HashSet<_>>();

    let input_builder = input::<usize>()
        .msg(&message)
        .add_test(move |value| test_values.contains(value))
        .err("Please enter the number of a selection from the list.")
        .repeat_msg(&message);

    let value = if let Some(default) = default {
        if let Some(index) = options.iter().position(|e| e.eq(default)) {
            input_builder.default(index + 1).get()
        } else {
            input_builder.get()
        }
    } else {
        input_builder.get()
    };

    Some(Value::String(choices.get(&value).unwrap().to_owned()))
}


pub fn render_answers(
    archetect: &Archetect,
    answers: &LinkedHashMap<String, AnswerInfo>,
    context: &Context,
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
