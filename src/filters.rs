use std::collections::HashMap;
use tera::{Result};
use serde_json::value::{to_value, Value};
use heck::{CamelCase, MixedCase, TitleCase, SnakeCase, ShoutySnakeCase, KebabCase};

#[macro_export]
macro_rules! try_get_value {
    ($filter_name:expr, $var_name:expr, $ty:ty, $val:expr) => {{
        match serde_json::from_value::<$ty>($val.clone()) {
            Ok(s) => s,
            Err(_) => {
                if $var_name == "value" {
                    return Err(tera::Error::msg(format!(
                        "Filter `{}` was called on an incorrect value: got `{}` but expected a {}",
                        $filter_name, $val, stringify!($ty)
                    )));
                } else {
                    return Err(tera::Error::msg(format!(
                        "Filter `{}` received an incorrect type for arg `{}`: got `{}` but expected a {}",
                        $filter_name, $var_name, $val, stringify!($ty)
                    )));
                }
            }
        }
    }};
}

pub fn pascal_case(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
    let s = try_get_value!("pascal_case", "value", String, value);
    Ok(to_value(&s.to_camel_case()).unwrap())
}

pub fn camel_case(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
    let s = try_get_value!("camel_case", "value", String, value);
    Ok(to_value(&s.to_mixed_case()).unwrap())
}

pub fn title_case(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
    let s = try_get_value!("title_case", "value", String, value);
    Ok(to_value(&s.to_title_case()).unwrap())
}

pub fn train_case(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
    let s = try_get_value!("train_case", "value", String, value);
    Ok(to_value(&s.to_kebab_case()).unwrap())
}

pub fn snake_case(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
    let s = try_get_value!("snake_case", "value", String, value);
    Ok(to_value(&s.to_snake_case()).unwrap())
}

pub fn constant_case(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
    let s = try_get_value!("constant_case", "value", String, value);
    Ok(to_value(&s.to_shouty_snake_case()).unwrap())
}

pub fn package_to_directory(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
    let s = try_get_value!("constant_case", "value", String, value);
    Ok(to_value(&s.replace(".", "/")).unwrap())
}

pub fn directory_to_package(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
    let s = try_get_value!("constant_case", "value", String, value);
    Ok(to_value(&s.replace("/", ".")).unwrap())
}


