/// Filters operating on string
use std::collections::HashMap;

use crate::vendor::heck::{
    CamelCase, ConstantCase, DirectoryCase, PackageCase, PascalCase, SnakeCase, TitleCase, TrainCase,
};
use crate::vendor::tera::{Result, Tera};
use serde_json::value::{to_value, Value};

use crate::try_get_value;

pub fn apply_filters(tera: &mut Tera) {
    tera.register_filter("pascal_case", crate::tera::filters::pascal_case);
    tera.register_filter("PascalCase", crate::tera::filters::pascal_case);
    tera.register_filter("camel_case", crate::tera::filters::camel_case);
    tera.register_filter("camelCase", crate::tera::filters::camel_case);
    tera.register_filter("title_case", crate::tera::filters::title_case);
    tera.register_filter("train_case", crate::tera::filters::train_case);
    tera.register_filter("train-case", crate::tera::filters::train_case);
    tera.register_filter("snake_case", crate::tera::filters::snake_case);
    tera.register_filter("constant_case", crate::tera::filters::constant_case);
    tera.register_filter("CONSTANT_CASE", crate::tera::filters::constant_case);
    tera.register_filter("directory_case", crate::tera::filters::directory_case);
    tera.register_filter("package_case", crate::tera::filters::package_case);
    tera.register_filter("package_to_directory", crate::tera::filters::package_to_directory);
    tera.register_filter("directory_to_package", crate::tera::filters::directory_to_package);

    tera.register_filter("pluralize", crate::tera::filters::pluralize);
    tera.register_filter("singularize", crate::tera::filters::singularize);
    tera.register_filter("ordinalize", crate::tera::filters::ordinalize);

    tera.register_filter("upper_case", crate::vendor::tera::builtins::filters::upper);
    tera.register_filter("lower_case", crate::tera::filters::lower);
}

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

//noinspection DuplicatedCode
/// Convert a value to uppercase.
pub fn upper(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
    let s = try_get_value!("upper", "value", String, value);

    Ok(to_value(&s.to_uppercase()).unwrap())
}

//noinspection DuplicatedCode
/// Convert a value to lowercase.
pub fn lower(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
    let s = try_get_value!("lower", "value", String, value);

    Ok(to_value(&s.to_lowercase()).unwrap())
}
