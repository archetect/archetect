use rhai::{Dynamic, Engine, EvalAltResult, Map};
use serde_json::Value;

pub fn register(engine: &mut Engine) {
    engine.register_fn("as_json", as_json);
    engine.register_fn("as_yaml", as_yaml);
    engine.register_fn("as_rhai", as_rhai);
    engine.register_fn("from_yaml", from_yaml);
    engine.register_fn("from_json", from_json);
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

pub fn from_yaml(value: Dynamic) -> Result<Dynamic, Box<EvalAltResult>> {
    let text = value.to_string();
    match serde_yaml::from_str::<Map>(&text) {
        Ok(map) => Ok(map.into()),
        Err(err) => Err(Box::new(EvalAltResult::ErrorSystem(
            "from_yaml Error".into(),
            Box::new(err),
        ))),
    }
}

pub fn from_json(value: Dynamic) -> Result<Dynamic, Box<EvalAltResult>> {
    let text = value.to_string();
    match serde_json::from_str::<Map>(&text) {
        Ok(map) => Ok(map.into()),
        Err(err) => Err(Box::new(EvalAltResult::ErrorSystem(
            "from_json Error".into(),
            Box::new(err),
        ))),
    }
}
