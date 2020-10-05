use std::collections::{HashMap, HashSet};

use linked_hash_map::LinkedHashMap;
use log::{trace, warn};
use read_input::prelude::*;
use serde_json::Value;

use crate::config::{AnswerInfo, VariableInfo, VariableType};
use crate::template_engine::Context;
use crate::{Archetect, ArchetectError};

const ACCEPTABLE_BOOLEANS: [&str; 8] = ["y", "yes", "true", "t", "n", "no", "false", "f"];

pub fn populate_context(
    archetect: &Archetect,
    variables: &LinkedHashMap<String, VariableInfo>,
    answers: &LinkedHashMap<String, AnswerInfo>,
    context: &mut Context,
) -> Result<(), ArchetectError> {
    for (identifier, variable_info) in variables {
        if let Some(answer) = answers.get(identifier) {
            if let Some(value) = answer.value() {
                // If there is an answer for this variable, it has an explicit value, and it is an acceptable answer,
                // use that.
                if insert_answered_variable(archetect, identifier, value, &variable_info.variable_type(), context)? {
                    continue;
                }
            }
        } else {
            if let Some(value) = variable_info.value() {
                // If no answer was provided, there is an explicit value on the variable definition, and it is an
                // acceptable value, use that.
                if insert_answered_variable(archetect, identifier, value, &variable_info.variable_type(), context)? {
                    continue;
                }
            }
        }

        trace!("Attempting to satisfy {} ({:?})", identifier, variable_info);

        // If we've made it this far, there was not an acceptable answer or explicit value.  We need to prompt for a
        // valid value
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
            VariableType::Enum(values) => prompt_for_enum(&mut prompt, &values, &default),
            VariableType::Bool => prompt_for_bool(&mut prompt, &default),
            VariableType::Int => prompt_for_int(&mut prompt, &default),
            VariableType::Array => prompt_for_list(archetect, context, &prompt, variable_info)?,
            VariableType::String => prompt_for_string(&mut prompt, &default, variable_info.required()),
        };

        if let Some(value) = value {
            context.insert(identifier, &value);
        }
    }

    Ok(())
}

fn insert_answered_variable(archetect: &Archetect, identifier: &str, value: &str, variable_type: &VariableType,
                            context: &mut Context) -> Result<bool, ArchetectError> {

    trace!("Setting variable answer {:?}={:?}", identifier, value);
    
    match variable_type {
        VariableType::Enum(options) => {
            // If the provided answer matches one of the enum values, use that; otherwise, we'll have to
            // prompt the user for a valid answer
            if options.contains(&value.to_owned()) {
                context.insert(identifier, &archetect.render_string(value, context)?);
                return Ok(true);
            }
        }
        VariableType::Bool => {
            let value = value.to_lowercase();
            // If the provided answer is anything that resembled a boolean value, use that; otherwise, we'll
            // have to prompt the user for a valid answer
            if ACCEPTABLE_BOOLEANS.contains(&value.as_str()) {
                let value = match ACCEPTABLE_BOOLEANS.iter().position(|i| i == &value.as_str()).unwrap() {
                    0..=3 => true,
                    _ => false,
                };
                context.insert(identifier, &value);
                return Ok(true);
            }
        }
        VariableType::Int => {
            // If the provided answer parses to an integer, use that; otherwise, we'll have to prompt the
            // user for a proper integer
            if let Ok(value) = &archetect.render_string(value, context)?.parse::<i64>() {
                context.insert(identifier, &value);
                return Ok(true);
            } else {
                trace!("'{}' failed to parse as an int", value);
            }
        }
        VariableType::String => {
            context.insert(identifier, &archetect.render_string(value, context)?);
            return Ok(true);
        }
        VariableType::Array => {
            context.insert(identifier, &archetect.render_string(value, context)?);
            return Ok(true);
        }
    }

    warn!("'{:?}' is not a valid answer for {:?} with type {:?}", value, identifier, variable_type);
    return Ok(false);
}

fn prompt_for_string(prompt: &mut String, default: &Option<String>, required: bool) -> Option<Value> {
    if let Some(default) = &default {
        prompt.push_str(format!("[{}] ", default).as_str());
    };
    let mut input_builder = input::<String>().msg(&prompt);

    if required {
        input_builder = input_builder
            .add_test(|value| value.len() > 0)
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
        .repeat_msg(&prompt);

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
        .add_test(|value| ACCEPTABLE_BOOLEANS.contains(&value.to_lowercase().as_str()))
        .msg(&prompt)
        .err(format!("Please specify a value of {:?}.", ACCEPTABLE_BOOLEANS))
        .repeat_msg(&prompt);

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
        let mut input_builder = input::<String>().msg("Item: ");

        if variable_info.required() {
            input_builder = input_builder
                .add_test(move |value| count > 0 || !value.trim().is_empty())
                .err("This list requires at least one item.")
                .repeat_msg(" - ")
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum VariableDescriptor {
    #[serde(rename = "object!")]
    Object {
        #[serde(skip_serializing_if = "Option::is_none")]
        prompt: Option<String>,
        //        #[serde(flatten)]
        items: LinkedHashMap<String, Box<VariableDescriptor>>,
    },

    #[serde(rename = "array!")]
    Array {
        prompt: String,
        //        #[serde(flatten)]
        item: Box<VariableDescriptor>,
    },

    #[serde(rename = "string!")]
    String {
        prompt: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        default: Option<String>,
    },

    #[serde(rename = "enum!")]
    Enum {
        prompt: String,
        options: Vec<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        default: Option<String>,
    },

    #[serde(rename = "bool!")]
    Bool {
        prompt: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        default: Option<String>,
    },

    #[serde(rename = "number!")]
    Number {
        prompt: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        default: Option<String>,
    },

    #[serde(rename = "json!")]
    Json { content: String, render: Option<bool> },
}

#[cfg(test)]
mod tests {
    use crate::actions::set::VariableDescriptor;
    use linked_hash_map::LinkedHashMap;

    #[test]
    fn test_serialize() {
        let object = VariableDescriptor::Object {
            prompt: Some("Schema:".to_string()),
            items: values_map(vec![(
                "tables",
                VariableDescriptor::Array {
                    prompt: "Tables: ".to_string(),
                    item: Box::new(VariableDescriptor::Object {
                        prompt: None,
                        items: values_map(vec![
                            (
                                "name",
                                VariableDescriptor::String {
                                    prompt: "Table Name: ".to_string(),
                                    default: None,
                                },
                            ),
                            (
                                "fields",
                                VariableDescriptor::Array {
                                    prompt: "Fields: ".to_string(),
                                    item: Box::new(VariableDescriptor::Object {
                                        prompt: None,
                                        items: values_map(vec![
                                            (
                                                "type",
                                                VariableDescriptor::Enum {
                                                    prompt: "Field Type: ".to_string(),
                                                    options: vec!["String".to_owned(), "Integer".to_owned()],
                                                    default: Some("String".to_owned()),
                                                },
                                            ),
                                            (
                                                "name",
                                                VariableDescriptor::String {
                                                    prompt: "Field Name: ".to_string(),
                                                    default: None,
                                                },
                                            ),
                                        ]),
                                    }),
                                },
                            ),
                        ]),
                    }),
                },
            )]),
        };

        let yaml = serde_yaml::to_string(&object).unwrap();
        println!("{}", yaml);
    }

    fn values_map<K: Into<String>, V>(values: Vec<(K, V)>) -> LinkedHashMap<String, Box<V>> {
        let mut results = LinkedHashMap::new();
        for (identifier, value) in values {
            results.insert(identifier.into(), Box::new(value));
        }
        results
    }
}
