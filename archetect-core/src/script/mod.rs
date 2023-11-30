use std::borrow::Cow;


use uuid::Uuid;

use minijinja::{Environment, Source, UndefinedBehavior};

use crate::archetype::archetype::Archetype;
use crate::archetype::render_context::RenderContext;
use crate::runtime::context::RuntimeContext;
use crate::script::rhai::modules::cases::{to_cobol_case, to_directory_case, to_package_case};

pub mod rhai;

pub(crate) fn create_environment(archetype: &Archetype, _runtime_context: RuntimeContext, render_context: &RenderContext) -> Environment<'static> {
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

    let templates = archetype.template_directory();

    if templates.exists() {
        environment.set_source(Source::from_path(templates));
    }

    let switches = render_context.switches().clone();
    environment.add_function("switch_enabled", move |switch: Cow<'_, str>| switches.contains(switch.as_ref()));
    environment
}