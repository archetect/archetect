/// Filters operating on string
use std::collections::HashMap;

use crate::vendor::tera::{Result, Tera};
use serde_json::value::{to_value, Value};

use crate::try_get_value;
use crate::v2::script::rhai::modules::cases;

pub fn apply_filters(tera: &mut Tera) {
    tera.register_filter("pascal_case", pascal_case);
    tera.register_filter("PascalCase", pascal_case);
    tera.register_filter("camel_case", camel_case);
    tera.register_filter("camelCase", camel_case);
    tera.register_filter("title_case", title_case);
    tera.register_filter("train_case", train_case);
    tera.register_filter("train-case", train_case);
    tera.register_filter("snake_case", snake_case);
    tera.register_filter("constant_case", constant_case);
    tera.register_filter("CONSTANT_CASE", constant_case);
    tera.register_filter("directory_case", directory_case);
    tera.register_filter("package_case", package_case);
    tera.register_filter("package_to_directory", package_to_directory);
    tera.register_filter("directory_to_package", directory_to_package);

    tera.register_filter("pluralize", pluralize);
    tera.register_filter("singularize", singularize);
    tera.register_filter("ordinalize", ordinalize);

    tera.register_filter("upper_case", crate::vendor::tera::builtins::filters::string::upper);
    tera.register_filter("lower_case", crate::vendor::tera::builtins::filters::string::lower);
}

pub fn pascal_case(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
    let s = try_get_value!("pascal_case", "value", String, value);
    Ok(to_value(cruet::to_pascal_case(&s)).unwrap())
}

pub fn camel_case(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
    let s = try_get_value!("camel_case", "value", String, value);
    Ok(to_value(cruet::to_camel_case(&s)).unwrap())
}

pub fn title_case(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
    let s = try_get_value!("title_case", "value", String, value);
    Ok(to_value(cruet::to_title_case(&s)).unwrap())
}

pub fn train_case(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
    let s = try_get_value!("train_case", "value", String, value);
    Ok(to_value(cruet::to_kebab_case(&s)).unwrap())
}

pub fn snake_case(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
    let s = try_get_value!("snake_case", "value", String, value);
    Ok(to_value(cruet::to_snake_case(&s)).unwrap())
}

pub fn constant_case(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
    let s = try_get_value!("constant_case", "value", String, value);
    Ok(to_value(cruet::to_screaming_snake_case(&s)).unwrap())
}

pub fn package_case(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
    let s = try_get_value!("package_case", "value", String, value);
    Ok(to_value(cases::to_package_case(&s)).unwrap())
}

pub fn directory_case(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
    let s = try_get_value!("directory_case", "value", String, value);
    Ok(to_value(cases::to_directory_case(&s)).unwrap())
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
    let plural = cruet::to_plural(&input);
    Ok(to_value(plural).unwrap())
}

pub fn singularize(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
    let input = try_get_value!("singularize", "value", String, value);
    let singular = cruet::to_singular(&input);
    Ok(to_value(singular).unwrap())
}

pub fn ordinalize(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
    let input = try_get_value!("ordinalize", "value", String, value);
    let plural = cruet::ordinalize(&input);
    Ok(to_value(plural).unwrap())
}
