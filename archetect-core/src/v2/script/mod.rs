use crate::v2::runtime::context::RuntimeContext;
use crate::v2::script::rhai::modules::cases::{to_cobol_case, to_directory_case, to_package_case};
use minijinja::{Environment, Source, UndefinedBehavior};
use std::borrow::Cow;
use uuid::Uuid;
use crate::v2::archetype::archetype::Archetype;

pub mod rhai;

pub(crate) fn create_environment(runtime_context: RuntimeContext, archetype: &Archetype) -> Environment<'static> {
    let mut environment = Environment::new();
    environment.set_undefined_behavior(UndefinedBehavior::Strict);
    environment.add_filter("camel_case", |value: Cow<'_, str>| cruet::to_camel_case(value.as_ref()));
    environment.add_filter("class_case", |value: Cow<'_, str>| cruet::to_class_case(value.as_ref()));
    environment.add_filter("cobol_case", |value: Cow<'_, str>| to_cobol_case(value.as_ref()));
    environment.add_filter("constant_case", |value: Cow<'_, str>| {
        cruet::to_screaming_snake_case(value.as_ref())
    });
    environment.add_filter("directory_case", |value: Cow<'_, str>| {
        to_directory_case(value.as_ref())
    });
    environment.add_filter("kebab_case", |value: Cow<'_, str>| cruet::to_kebab_case(value.as_ref()));
    environment.add_filter("lower_case", |value: Cow<'_, str>| str::to_lowercase(value.as_ref()));
    environment.add_filter("pascal_case", |value: Cow<'_, str>| {
        cruet::to_pascal_case(value.as_ref())
    });
    environment.add_filter("package_case", |value: Cow<'_, str>| to_package_case(value.as_ref()));
    environment.add_filter("sentence_case", |value: Cow<'_, str>| {
        cruet::to_sentence_case(value.as_ref())
    });
    environment.add_filter("snake_case", |value: Cow<'_, str>| cruet::to_snake_case(value.as_ref()));
    environment.add_filter("train_case", |value: Cow<'_, str>| cruet::to_train_case(value.as_ref()));
    environment.add_filter("title_case", |value: Cow<'_, str>| cruet::to_title_case(value.as_ref()));
    environment.add_filter("upper_case", |value: Cow<'_, str>| str::to_uppercase(value.as_ref()));

    environment.add_filter("pluralize", |value: Cow<'_, str>| cruet::to_plural(value.as_ref()));
    environment.add_filter("plural", |value: Cow<'_, str>| cruet::to_plural(value.as_ref()));
    environment.add_filter("singularize", |value: Cow<'_, str>| cruet::to_singular(value.as_ref()));
    environment.add_filter("singular", |value: Cow<'_, str>| cruet::to_singular(value.as_ref()));

    environment.add_filter("ordinalize", |value: Cow<'_, str>| cruet::ordinalize(value.as_ref()));
    environment.add_filter("deordinalize", |value: Cow<'_, str>| {
        cruet::deordinalize(value.as_ref())
    });

    environment.add_function("uuid", || Uuid::new_v4().to_string());

    let templates = archetype.root().join("templates");

    if templates.exists() {
        environment.set_source(Source::from_path(templates));
    }

    let rc = runtime_context.clone();
    environment.add_function("switch_enabled", move |switch: Cow<'_, str>| rc.switch_enabled(switch));
    environment
}
