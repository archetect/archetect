use crate::errors::{ArchetypeScriptError, ArchetypeScriptErrorWrapper};
use crate::script::rhai::modules::cases_module::{expand_key_value_cases, extract_case_strategies};
use crate::script::rhai::modules::prompt_module::Caseable;
use rhai::{Dynamic, Engine, EvalAltResult, Map, NativeCallContext};

const SET_METHOD: &'static str = "set";
const CASED_AS: &'static str = "cased_as";

pub(crate) fn register(engine: &mut Engine) {
    engine.register_fn(
        SET_METHOD,
        move |call: NativeCallContext, key: &str, value: Dynamic| set(&call, key, value, Map::new()),
    );

    engine.register_fn(
        SET_METHOD,
        move |call: NativeCallContext, key: &str, value: Dynamic, settings: Map| set(&call, key, value, settings),
    );
}

fn set(call: &NativeCallContext, key: &str, value: Dynamic, settings: Map) -> Result<Map, Box<EvalAltResult>> {
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
    results.insert(key.into(), value.clone_cast());

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
