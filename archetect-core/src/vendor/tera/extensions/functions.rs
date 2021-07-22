use crate::vendor::tera::errors::Result;
use crate::vendor::tera::{Tera, Value};
use std::collections::HashMap;

pub fn apply_functions(tera: &mut Tera) {
    tera.register_function("uuid", uuid);
}

pub fn uuid(_args: &HashMap<String, Value>) -> Result<Value> {
    let id = uuid::Uuid::new_v4();
    Ok(Value::from(id.to_string()))
}
