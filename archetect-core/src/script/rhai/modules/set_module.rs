use crate::errors::{ArchetypeScriptError, ArchetypeScriptErrorWrapper};
use crate::script::rhai::modules::cases_module::{expand_key_value_cases, extract_case_strategies};
use crate::script::rhai::modules::prompt_module::{Caseable, RequirementDescription};
use rhai::{Dynamic, Engine, EvalAltResult, Map, NativeCallContext};
use crate::archetype::render_context::RenderContext;

const SET_METHOD: &'static str = "set";
const CASED_AS: &'static str = "cased_as";

pub(crate) fn register(engine: &mut Engine, render_context: RenderContext) {
    let rc_clone = render_context.clone();
    engine.register_fn(
        SET_METHOD,
        move |call: NativeCallContext, key: &str, value: Dynamic| set(&call, rc_clone.clone(), key, value, Map::new()),
    );

    let rc_clone = render_context.clone();
    engine.register_fn(
        SET_METHOD,
        move |call: NativeCallContext, key: &str, value: Dynamic, settings: Map| set(&call, rc_clone.clone(), key, value, settings),
    );
}

fn set(call: &NativeCallContext, render_context: RenderContext, key: &str, mut value: Dynamic, settings: Map) -> Result<Map, Box<EvalAltResult>> {
    let allow_answer = cast_setting("allow_answer", &settings, key)
        .map_err(|err| ArchetypeScriptErrorWrapper(call, err))?
        .unwrap_or_default();
    if allow_answer {
        let answers = get_answers(call, &settings, key, &render_context)?;

        if let Some(answer) = answers.get(key) {
            value = answer.clone();
        }
    }

    let case_strategies = extract_case_strategies(&settings).map_err(|err| {
        ArchetypeScriptErrorWrapper(
            call,
            ArchetypeScriptError::KeyedInvalidSetSetting {
                setting: CASED_AS.to_string(),
                requirement: err,
                key: key.to_string(),
            },
        )
    })?;
    let mut results: Map = Map::new();

    if value.is_string() {
        expand_key_value_cases(&case_strategies, &mut results, key, Caseable::String(value.to_string()));
    } else if value.is_array() {
        let list = value
            .into_array()
            .expect("Prechecked")
            .into_iter()
            .map(|v| v.to_string())
            .collect::<Vec<String>>();
        expand_key_value_cases(&case_strategies, &mut results, key, Caseable::List(list));
    }

    Ok(results)
}

fn get_answers<'a, K: AsRef<str> + Clone>(
    call: &NativeCallContext,
    settings: &Map,
    key: K,
    render_context: &RenderContext,
) -> Result<Map, Box<EvalAltResult>> {
    let setting = "answer_source";
    if let Some(answers) = settings.get(setting) {
        return if let Some(answers) = answers.clone().try_cast::<Map>() {
            Ok(answers)
        } else {
            if !answers.is_unit() {
                let requirement = "a 'map' (\"#{{ .. }}\") or Unit type (\"()\")".to_string();
                let error = ArchetypeScriptError::KeyedInvalidSetSetting {
                    setting: "answers".to_string(),
                    requirement,
                    key: key.as_ref().to_string(),
                };
                Err(ArchetypeScriptErrorWrapper(call, error).into())
            } else {
                Ok(Map::new())
            }
        };
    }
    let answers = render_context.answers();
    let mut results = Map::new();
    results.extend(answers.clone());
    return Ok(results);
}

pub fn cast_setting<'a, T, K>(
    setting: &str,
    settings: &Map,
    key: K,
) -> Result<Option<T>, ArchetypeScriptError>
    where
        K: AsRef<str>,
        T: RequirementDescription + 'static,
{
    return match settings.get(setting) {
        None => Ok(None),
        Some(value) => {
            if let Some(value) = value.clone().try_cast::<T>() {
                return Ok(Some(value));
            } else if value.is_unit() {
                return Ok(None);
            }
            Err(ArchetypeScriptError::KeyedInvalidSetSetting {
                setting: setting.to_string(),
                requirement: T::get_requirement().to_string(),
                key: key.as_ref().to_string(),
            })
        }
    };
}