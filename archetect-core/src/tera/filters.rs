/// Filters operating on string
use std::collections::HashMap;

use crate::vendor::heck::{
    CamelCase, ConstantCase, DirectoryCase, PackageCase, PascalCase, SnakeCase, TitleCase, TrainCase,
};
use crate::vendor::tera::Result;
use serde_json::value::{to_value, Value};

use crate::try_get_value;

pub fn pascal_case(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
    let s = try_get_value!("pascal_case", "value", String, value);
    Ok(to_value(&s.to_pascal_case()).unwrap())
}

pub fn camel_case(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
    let s = try_get_value!("camel_case", "value", String, value);
    Ok(to_value(&s.to_camel_case()).unwrap())
}

pub fn title_case(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
    let s = try_get_value!("title_case", "value", String, value);
    Ok(to_value(&s.to_title_case()).unwrap())
}

pub fn train_case(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
    let s = try_get_value!("train_case", "value", String, value);
    Ok(to_value(&s.to_train_case()).unwrap())
}

pub fn snake_case(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
    let s = try_get_value!("snake_case", "value", String, value);
    Ok(to_value(&s.to_snake_case()).unwrap())
}

pub fn constant_case(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
    let s = try_get_value!("constant_case", "value", String, value);
    Ok(to_value(&s.to_constant_case()).unwrap())
}

pub fn package_case(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
    let s = try_get_value!("package_case", "value", String, value);
    Ok(to_value(&s.to_package_case()).unwrap())
}

pub fn directory_case(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
    let s = try_get_value!("directory_case", "value", String, value);
    Ok(to_value(&s.to_directory_case()).unwrap())
}

pub fn package_to_directory(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
    let s = try_get_value!("constant_case", "value", String, value);
    Ok(to_value(&s.replace(".", "/")).unwrap())
}

pub fn directory_to_package(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
    let s = try_get_value!("constant_case", "value", String, value);
    Ok(to_value(&s.replace("/", ".")).unwrap())
}

pub fn pluralize(value: &Value, _args: &HashMap<String, Value>) -> Result<Value> {
    let input = try_get_value!("pluralize", "value", String, value);
    let plural = inflector::string::pluralize::to_plural(&input);
    Ok(to_value(plural).unwrap())
}

pub fn singularize(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
    let input = try_get_value!("singularize", "value", String, value);
    let singular = inflector::string::singularize::to_singular(&input);
    Ok(to_value(singular).unwrap())
}

pub fn ordinalize(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
    let input = try_get_value!("ordinalize", "value", String, value);
    let plural = inflector::numbers::ordinalize::ordinalize(&input);
    Ok(to_value(plural).unwrap())
}

/// Convert a value to uppercase.
pub fn upper(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
    let s = try_get_value!("upper", "value", String, value);

    Ok(to_value(&s.to_uppercase()).unwrap())
}

/// Convert a value to lowercase.
pub fn lower(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
    let s = try_get_value!("lower", "value", String, value);

    Ok(to_value(&s.to_lowercase()).unwrap())
}
