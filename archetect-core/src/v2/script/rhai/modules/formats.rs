use rhai::{Dynamic, Engine, EvalAltResult};
use std::result;
use rhai::plugin::RhaiResult;
use serde_json::Value;

pub fn register(engine: &mut Engine) {
    engine.register_fn("as_json", as_json);
    engine.register_fn("as_yaml", as_yaml);
    engine.register_fn("as_rhai", as_rhai);
}

pub fn as_json(value: Dynamic) -> Result<String, Box<EvalAltResult>> {
    let value: Value = rhai::serde::from_dynamic(&value)?;

    serde_json::to_string_pretty(&value)
        .map_err(|err| Box::new(EvalAltResult::ErrorSystem("to_json Error".into(), Box::new(err))))
}

pub fn as_yaml(value: Dynamic) -> Result<String, Box<EvalAltResult>> {
    let result = serde_yaml::to_string(&value);
    match result {
        Ok(value) => Ok(value),
        Err(err) => Err(Box::new(EvalAltResult::ErrorSystem(
            "to_yaml Error".into(),
            Box::new(err),
        ))),
    }
}

pub fn as_rhai(value: Dynamic) -> Result<String, Box<EvalAltResult>> {
    Ok(format!("{}", value))
}
