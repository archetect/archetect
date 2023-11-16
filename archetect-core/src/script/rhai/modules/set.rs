use rhai::{Dynamic, Engine, EvalAltResult, Map};
use crate::script::rhai::modules::cases::{expand_key_value_cases};

pub(crate) fn register(engine: &mut Engine) {
    engine.register_fn("set", | key: &str, value: Dynamic| {
        set(key, value, Map::new())
    });

    engine.register_fn("set", | key: &str, value: Dynamic, settings: Map | {
        set(key, value, settings)
    });
}

fn set(key: &str, value: Dynamic, settings: Map) -> Result<Map, Box<EvalAltResult>> {
    let mut results: Map = Map::new();
    results.insert(key.into(), value.clone_cast());
    expand_key_value_cases(&settings, &mut results, key, value.to_string().as_str());
    Ok(results)
}
