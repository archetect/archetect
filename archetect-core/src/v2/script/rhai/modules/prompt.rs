use std::ops::{RangeFrom, RangeInclusive, RangeToInclusive};

use log::warn;
use rhai::plugin::*;
use rhai::{exported_module, Dynamic, Engine, EvalAltResult, Map};

use inquire::validator::Validation;
use inquire::{Confirm, InquireError, MultiSelect, Select, Text};

use crate::v2::archetype::archetype::Archetype;
use crate::v2::archetype::archetype_context::ArchetypeContext;
use crate::v2::runtime::context::RuntimeContext;
use crate::v2::script::rhai::modules::cases::expand_cases;
use crate::{ArchetectError, ArchetypeError};

pub(crate) fn register(
    engine: &mut Engine,
    archetype: Archetype,
    archetype_context: ArchetypeContext,
    runtime_context: RuntimeContext,
) {
    engine.register_global_module(exported_module!(module).into());

    let arch = archetype.clone();
    let ctx = archetype_context.clone();
    let rt = runtime_context.clone();
    engine.register_fn("prompt", move |message: &str, key: &str, settings: Map| {
        prompt_to_map(arch.clone(), ctx.clone(), rt.clone(), message, key, settings)
    });

    let arch = archetype.clone();
    let ctx = archetype_context.clone();
    let rt = runtime_context.clone();
    engine.register_fn("prompt", move |message: &str, key: &str| {
        prompt_to_map(arch.clone(), ctx.clone(), rt.clone(), message, key, Map::new())
    });

    let rt = runtime_context.clone();
    engine.register_fn("prompt", move |message: &str, settings: Map| {
        prompt_to_value(message, rt.clone(), settings)
    });

    let rt = runtime_context.clone();
    engine.register_fn("prompt", move |message: &str| {
        prompt_to_value(message, rt.clone(), Map::new())
    });
}

fn prompt_to_map(
    _archetype: Archetype,
    archetype_context: ArchetypeContext,
    runtime_context: RuntimeContext,
    message: &str,
    key: &str,
    settings: Map,
) -> Result<Dynamic, Box<EvalAltResult>> {
    let prompt_type = get_prompt_type(&settings).map_err(|err| {
        Box::new(EvalAltResult::ErrorSystem(
            "Invalid PromptType".to_owned(),
            Box::new(err),
        ))
    })?;

    let mut results: Map = Map::new();

    let mut answers_provided = false;

    if let Some(answers) = settings.get("answers") {
        if let Some(answers) = answers.clone().try_cast::<Map>() {
            answers_provided = true;
            if let Some(answer) = answers.get(key) {
                results.insert(key.into(), answer.clone().into());
                expand_cases(&settings, &mut results, key, answer.to_string().as_str());
                return Ok(results.into());
            }
        } else {
            if !answers.is_unit() {
                warn!(
                    "Answers should be a Map, but were supplied as a {}",
                    answers.type_name()
                );
            }
        }
    };

    if !answers_provided {
        if let Some(answer) = archetype_context.answers().get(key) {
            results.insert(key.into(), answer.clone().into());
            expand_cases(&settings, &mut results, key, answer.to_string().as_str());
            return Ok(results.into());
        };
    }

    match prompt_type {
        PromptType::Text => {
            let value = prompt_text(message, &settings, &runtime_context)?;
            results.insert(key.into(), value.clone().into());
            expand_cases(&settings, &mut results, key, &value);
            return Ok(results.into());
        }
        PromptType::Confirm => {
            let value = prompt_confirm(message, &runtime_context, &settings)?;
            results.insert(key.into(), value.into());
            return Ok(results.into());
        }
        PromptType::Int => {
            let value = prompt_int(message, &runtime_context, &settings)?;
            results.insert(key.into(), value.into());
            return Ok(results.into());
        }
        PromptType::Select(options) => {
            let value = prompt_select(message, options, &runtime_context, &settings)?;
            results.insert(key.into(), value.clone().into());
            expand_cases(&settings, &mut results, key, &value);
            return Ok(results.into());
        }
        PromptType::MultiSelect(options) => {
            let value = prompt_multiselect(message, options, &runtime_context, &settings)?;
            results.insert(key.into(), value.into());
            return Ok(results.into());
        }
        PromptType::Editor => {
            let value = prompt_editor(message)?;
            results.insert(key.into(), value.into());
            return Ok(results.into());
        }
        // PromptType::List => {}
        _ => panic!("Unimplemented PromptType"),
    }
}

fn prompt_editor(message: &str) -> Result<String, Box<EvalAltResult>> {
    let prompt = inquire::Editor::new(message);

    let result = prompt.prompt();
    match result {
        Ok(text) => Ok(text),
        Err(err) => match err {
            InquireError::OperationCanceled => {
                return Err(Box::new(EvalAltResult::ErrorSystem(
                    "Cancelled".to_owned(),
                    Box::new(ArchetypeError::ValueRequired),
                )));
            }
            InquireError::OperationInterrupted => {
                return Err(Box::new(EvalAltResult::ErrorSystem(
                    "Cancelled".to_owned(),
                    Box::new(ArchetypeError::OperationInterrupted),
                )));
            }
            err => Err(Box::new(EvalAltResult::ErrorSystem("Error".to_owned(), Box::new(err)))),
        },
    }
}

fn prompt_select(
    message: &str,
    options: Vec<Dynamic>,
    runtime_context: &RuntimeContext,
    settings: &Map,
) -> Result<String, Box<EvalAltResult>> {
    let mut prompt = Select::new(message, options);

    let _optional = settings
        .get("optional")
        .map_or(Ok(false), |value| value.as_bool())
        .unwrap_or(false);

    // TODO: Handle Defaults
    if runtime_context.headless() {
        return Err(Box::new(EvalAltResult::ErrorSystem(
            "Headless Mode Error".to_owned(),
            Box::new(ArchetectError::HeadlessNoDefault),
        )));
    }

    if let Some(page_size) = settings.get("page_size") {
        if let Some(page_size) = page_size.clone().try_cast::<i64>() {
            prompt.page_size = page_size as usize;
        } else {
            warn!(
                "Invalid data type used for 'page_size': {}; should be an integer",
                page_size.type_name()
            );
        }
    } else {
        prompt.page_size = 10;
    }

    if let Some(help_message) = settings.get("help") {
        prompt.help_message = Some(help_message.to_string());
    }

    let result = prompt.prompt();

    match result {
        Ok(selection) => Ok(selection.to_string()),
        Err(err) => match err {
            InquireError::OperationCanceled => {
                return Err(Box::new(EvalAltResult::ErrorSystem(
                    "Cancelled".to_owned(),
                    Box::new(ArchetypeError::ValueRequired),
                )));
            }
            InquireError::OperationInterrupted => {
                return Err(Box::new(EvalAltResult::ErrorSystem(
                    "Cancelled".to_owned(),
                    Box::new(ArchetypeError::OperationInterrupted),
                )));
            }
            err => Err(Box::new(EvalAltResult::ErrorSystem("Error".to_owned(), Box::new(err)))),
        },
    }
}

fn prompt_multiselect(
    message: &str,
    options: Vec<Dynamic>,
    runtime_context: &RuntimeContext,
    settings: &Map,
) -> Result<Vec<Dynamic>, Box<EvalAltResult>> {
    let mut prompt = MultiSelect::new(message, options);

    let _optional = settings
        .get("optional")
        .map_or(Ok(false), |value| value.as_bool())
        .unwrap_or(false);

    // TODO: Handle Defaults
    if runtime_context.headless() {
        return Err(Box::new(EvalAltResult::ErrorSystem(
            "Headless Mode Error".to_owned(),
            Box::new(ArchetectError::HeadlessNoDefault),
        )));
    }

    if let Some(page_size) = settings.get("page_size") {
        if let Some(page_size) = page_size.clone().try_cast::<i64>() {
            prompt.page_size = page_size as usize;
        } else {
            warn!(
                "Invalid data type used for 'page_size': {}; should be an integer",
                page_size.type_name()
            );
        }
    } else {
        prompt.page_size = 10;
    }

    if let Some(help_message) = settings.get("help") {
        prompt.help_message = Some(help_message.to_string());
    }

    let result = prompt.prompt();

    match result {
        Ok(selections) => Ok(selections),
        Err(err) => match err {
            InquireError::OperationCanceled => Err(Box::new(EvalAltResult::ErrorSystem(
                "Cancelled".to_owned(),
                Box::new(ArchetypeError::ValueRequired),
            ))),
            InquireError::OperationInterrupted => Err(Box::new(EvalAltResult::ErrorSystem(
                "Cancelled".to_owned(),
                Box::new(ArchetypeError::OperationInterrupted),
            ))),
            err => Err(Box::new(EvalAltResult::ErrorSystem("Error".to_owned(), Box::new(err)))),
        },
    }
}

fn prompt_confirm(message: &str, runtime_context: &RuntimeContext, settings: &Map) -> Result<bool, Box<EvalAltResult>> {
    let mut prompt = Confirm::new(message);

    let _optional = settings
        .get("optional")
        .map_or(Ok(false), |value| value.as_bool())
        .unwrap_or(false);

    if let Some(default_value) = settings.get("defaults_with") {
        if let Some(default_value) = default_value.clone().try_cast::<bool>() {
            if runtime_context.headless() {
                return Ok(default_value);
            } else {
                prompt.default = Some(default_value);
            }
        }
    }

    if runtime_context.headless() {
        return Err(Box::new(EvalAltResult::ErrorSystem(
            "Headless Mode Error".to_owned(),
            Box::new(ArchetectError::HeadlessNoDefault),
        )));
    }

    if let Some(placeholder) = settings.get("placeholder") {
        prompt.placeholder = Some(placeholder.to_string());
    }

    if let Some(help_message) = settings.get("help") {
        prompt.help_message = Some(help_message.to_string());
    }

    let result = prompt.prompt();

    match result {
        Ok(value) => Ok(value),
        Err(err) => match err {
            InquireError::OperationCanceled => {
                return Err(Box::new(EvalAltResult::ErrorSystem(
                    "Cancelled".to_owned(),
                    Box::new(ArchetypeError::ValueRequired),
                )));
            }
            InquireError::OperationInterrupted => {
                return Err(Box::new(EvalAltResult::ErrorSystem(
                    "Cancelled".to_owned(),
                    Box::new(ArchetypeError::OperationInterrupted),
                )));
            }
            err => Err(Box::new(EvalAltResult::ErrorSystem("Error".to_owned(), Box::new(err)))),
        },
    }
}

fn prompt_int(message: &str, runtime_context: &RuntimeContext, settings: &Map) -> Result<i64, Box<EvalAltResult>> {
    let mut text = Text::new(message);

    let _optional = settings
        .get("optional")
        .map_or(Ok(false), |value| value.as_bool())
        .unwrap_or(false);

    if let Some(default_value) = settings.get("defaults_with") {
        let default_value = default_value.to_string();
        match default_value.parse::<i64>() {
            Ok(value) => {
                if runtime_context.headless() {
                    return Ok(value);
                } else {
                    text.default = Some(default_value.to_string());
                }
            }
            Err(_) => warn!("Default for prompt should be an integer, but was ({})", default_value),
        }
    }

    if runtime_context.headless() {
        return Err(Box::new(EvalAltResult::ErrorSystem(
            "Headless Mode Error".to_owned(),
            Box::new(ArchetectError::HeadlessNoDefault),
        )));
    }

    if let Some(placeholder) = settings.get("placeholder") {
        text.placeholder = Some(placeholder.to_string());
    }

    if let Some(help_message) = settings.get("help") {
        text.help_message = Some(help_message.to_string());
    }

    let min = settings
        .get("min")
        .map(|value| value.to_string().parse::<i64>())
        .map(|value| value.ok())
        .flatten();

    let max = settings
        .get("max")
        .map(|value| value.to_string().parse::<i64>())
        .map(|value| value.ok())
        .flatten();

    let _optional = settings
        .get("optional")
        .map_or(Ok(false), |value| value.as_bool())
        .unwrap_or(false);

    let validator = move |input: &str| match input.parse::<i64>() {
        Ok(value) => {
            match (min, max) {
                (Some(start), Some(end)) => {
                    if !RangeInclusive::new(start, end).contains(&value) {
                        return Ok(Validation::Invalid(
                            format!("Answer must be between {} and {}", start, end).into(),
                        ));
                    }
                }
                (Some(start), None) => {
                    if !(RangeFrom { start }.contains(&value)) {
                        return Ok(Validation::Invalid(
                            format!("Answer must be greater than {}", start).into(),
                        ));
                    }
                }
                (None, Some(end)) => {
                    if !(RangeToInclusive { end }.contains(&value)) {
                        return Ok(Validation::Invalid(
                            format!("Answer must be less than or equal {}", end).into(),
                        ));
                    }
                }
                _ => (),
            };

            Ok(Validation::Valid)
        }
        Err(_) => Ok(Validation::Invalid("Answer must be an integer".into())),
    };

    text = text.with_validator(validator);

    let result = text.prompt();

    match result {
        Ok(value) => Ok(value.parse::<i64>().unwrap()),
        Err(err) => match err {
            InquireError::OperationCanceled => {
                return Err(Box::new(EvalAltResult::ErrorSystem(
                    "Cancelled".to_owned(),
                    Box::new(ArchetypeError::ValueRequired),
                )));
            }
            InquireError::OperationInterrupted => {
                return Err(Box::new(EvalAltResult::ErrorSystem(
                    "Cancelled".to_owned(),
                    Box::new(ArchetypeError::OperationInterrupted),
                )));
            }
            err => Err(Box::new(EvalAltResult::ErrorSystem("Error".to_owned(), Box::new(err)))),
        },
    }
}

fn prompt_text(message: &str, settings: &Map, runtime_context: &RuntimeContext) -> Result<String, Box<EvalAltResult>> {
    // TODO: Validate characters
    let validator = |input: &str| match input.len() > 0 {
        true => Ok(Validation::Valid),
        false => Ok(Validation::Invalid("You must supply at least one character".into())),
    };

    let mut text = Text::new(message).with_validator(validator);

    let _optional = settings
        .get("optional")
        .map_or(Ok(false), |value| value.as_bool())
        .unwrap_or(false);

    if let Some(default_value) = settings.get("defaults_with") {
        if runtime_context.headless() {
            return Ok(default_value.to_string());
        } else {
            text.default = Some(default_value.to_string());
        }
    }

    if runtime_context.headless() {
        return Err(Box::new(EvalAltResult::ErrorSystem(
            "Headless Mode Error".to_owned(),
            Box::new(ArchetectError::HeadlessNoDefault),
        )));
    }

    if let Some(placeholder) = settings.get("placeholder") {
        text.placeholder = Some(placeholder.to_string());
    }

    if let Some(help_message) = settings.get("help") {
        text.help_message = Some(help_message.to_string());
    }

    let result = text.prompt();

    match result {
        Ok(value) => Ok(value),
        Err(err) => match err {
            InquireError::OperationCanceled => {
                return Err(Box::new(EvalAltResult::ErrorSystem(
                    "Cancelled".to_owned(),
                    Box::new(ArchetypeError::ValueRequired),
                )));
            }
            InquireError::OperationInterrupted => {
                return Err(Box::new(EvalAltResult::ErrorSystem(
                    "Cancelled".to_owned(),
                    Box::new(ArchetypeError::OperationInterrupted),
                )));
            }
            err => Err(Box::new(EvalAltResult::ErrorSystem("Error".to_owned(), Box::new(err)))),
        },
    }
}

fn prompt_to_value(
    message: &str,
    runtime_context: RuntimeContext,
    settings: Map,
) -> Result<Dynamic, Box<EvalAltResult>> {
    let prompt_type = get_prompt_type(&settings).map_err(|err| {
        Box::new(EvalAltResult::ErrorSystem(
            "Invalid PromptType".to_owned(),
            Box::new(err),
        ))
    })?;

    match prompt_type {
        PromptType::Text => {
            let value = prompt_text(message, &settings, &runtime_context)?;
            Ok(value.into())
        }
        PromptType::Confirm => {
            let value = prompt_confirm(message, &runtime_context, &settings)?;
            Ok(value.into())
        }
        PromptType::Int => {
            let value = prompt_int(message, &runtime_context, &settings)?;
            Ok(value.into())
        }
        PromptType::Select(options) => {
            let value = prompt_select(message, options, &runtime_context, &settings)?;
            Ok(value.into())
        }
        PromptType::MultiSelect(options) => {
            let value = prompt_multiselect(message, options, &runtime_context, &settings)?;
            Ok(value.into())
        }
        _ => panic!("Unimplemented PromptType"),
    }
}

pub fn get_prompt_type(settings: &Map) -> Result<PromptType, ArchetypeError> {
    if let Some(prompt_type) = settings.get("type") {
        if let Some(prompt_type) = prompt_type.clone().try_cast::<PromptType>() {
            // TODO: Throw Error if a value was provided but it is NOT a PromptType
            return Ok(prompt_type);
        }
    }
    Ok(PromptType::Text)
}

#[derive(Clone, Debug)]
pub enum PromptType {
    Text,
    Confirm,
    Int,
    List,
    Select(Vec<Dynamic>),
    MultiSelect(Vec<Dynamic>),
    Editor,
}

#[allow(non_snake_case)]
#[allow(non_upper_case_globals)]
#[export_module]
pub mod module {
    use rhai::Dynamic;

    pub type PromptType = crate::v2::script::rhai::modules::prompt::PromptType;

    pub const Text: PromptType = PromptType::Text;
    pub const Confirm: PromptType = PromptType::Confirm;
    pub const Int: PromptType = PromptType::Int;
    pub const List: PromptType = PromptType::List;
    pub const Editor: PromptType = PromptType::Editor;

    pub fn Select(options: Vec<Dynamic>) -> PromptType {
        PromptType::Select(options)
    }

    pub fn MultiSelect(options: Vec<Dynamic>) -> PromptType {
        PromptType::MultiSelect(options)
    }
}
