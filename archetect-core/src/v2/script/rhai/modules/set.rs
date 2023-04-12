use rhai::{Dynamic, Engine, EvalAltResult, Map};
use crate::v2::script::rhai::modules::cases::{expand_cases};

pub (crate) fn register(engine: &mut Engine) {
    engine.register_fn("set", | key: &str, value: Dynamic| {
        set(key, value, Map::new())
    });

    engine.register_fn("set", | key: &str, value: Dynamic, settings: Map | {
        set(key, value, settings)
    });
}

fn set(key: &str, value: Dynamic, settings: Map) -> Result<Map, Box<EvalAltResult>> {
    let mut results: Map = Map::new();
    results.insert(key.into(), value.to_string().into());
    expand_cases(&settings, &mut results, key, value.to_string().as_str());
    Ok(results)
}
