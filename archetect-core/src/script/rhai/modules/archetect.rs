use crate::runtime::context::RuntimeContext;
use rhai::{Engine, Module};

pub(crate) fn register(engine: &mut Engine, runtime: RuntimeContext) {
    let mut module = Module::new();

    let rt = runtime.clone();
    module.set_native_fn("version", move || Ok(rt.archetect_version().to_string()));
    let rt = runtime.clone();
    module.set_native_fn("version_major", move || Ok(rt.archetect_version().major.to_string()));
    let rt = runtime.clone();
    module.set_native_fn("version_minor", move || Ok(rt.archetect_version().minor.to_string()));
    let rt = runtime.clone();
    module.set_native_fn("version_patch", move || Ok(rt.archetect_version().patch.to_string()));
    let rt = runtime.clone();
    module.set_native_fn("is_offline", move || Ok(rt.offline()));
    let rt = runtime.clone();
    module.set_native_fn("is_headless", move || Ok(rt.headless()));
    let rt = runtime.clone();
    module.set_native_fn("locals_enabled", move|| Ok(rt.locals().enabled()));

    engine.register_static_module("archetect", module.into());
}
