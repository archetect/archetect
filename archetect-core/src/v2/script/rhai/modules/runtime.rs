use crate::v2::runtime::context::RuntimeContext;
use rhai::{Engine, Module};
use std::sync::Arc;

pub(crate) fn register(engine: &mut Engine, runtime: RuntimeContext) {
    let runtime = runtime.clone();
    let mut module = Module::new();
    module.set_native_fn("version", move || Ok(runtime.archetect_version().to_string()));
    engine.register_static_module("archetect", module.into());
}
