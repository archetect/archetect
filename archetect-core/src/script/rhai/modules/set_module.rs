use crate::script::rhai::modules::cases_module::expand_key_value_cases;
use rhai::{Dynamic, Engine, EvalAltResult, Map};
use crate::script::rhai::modules::prompt_module::Caseable;

pub(crate) fn register(engine: &mut Engine) {
    engine.register_fn("set", |key: &str, value: Dynamic| set(key, value, Map::new()));

    engine.register_fn("set", |key: &str, value: Dynamic, settings: Map| {
        set(key, value, settings)
    });
}
fn set(key: &str, value: Dynamic, settings: Map) -> Result<Map, Box<EvalAltResult>> {
    let mut results: Map = Map::new();
    results.insert(key.into(), value.clone_cast());
    if value.is_string() {
        expand_key_value_cases(&settings, &mut results, key, Caseable::String(value.to_string()));
    } else if value.is_array() {
        let list = value.into_array().expect("Prechecked")
            .into_iter()
            .map(|v|v.to_string())
            .collect::<Vec<String>>();
        expand_key_value_cases(&settings, &mut results, key, Caseable::List(list));
    }

    Ok(results)
}
