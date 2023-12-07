use rhai::{Dynamic, Engine, EvalAltResult, Module, NativeCallContext};
use rhai::module_resolvers::{FileModuleResolver, ModuleResolversCollection, StaticModuleResolver};

use crate::archetype::archetype::Archetype;
use crate::runtime::context::RuntimeContext;

pub(crate) fn register(engine: &mut Engine, runtime_context: RuntimeContext, archetype: Archetype) {
    let mut module = Module::new();

    let rt = runtime_context.clone();
    module.set_native_fn("version", move || Ok(rt.archetect_version().to_string()));
    let rt = runtime_context.clone();
    module.set_native_fn("version_major", move || Ok(rt.archetect_version().major.to_string()));
    let rt = runtime_context.clone();
    module.set_native_fn("version_minor", move || Ok(rt.archetect_version().minor.to_string()));
    let rt = runtime_context.clone();
    module.set_native_fn("version_patch", move || Ok(rt.archetect_version().patch.to_string()));

    let archetype_module = archetype_module(archetype.clone());
    module.set_sub_module("archetype", archetype_module.clone());
    let runtime_module = runtime_module(runtime_context.clone());
    module.set_sub_module("runtime", runtime_module.clone());

    let mut resolver = ModuleResolversCollection::new();

    let mut static_resolver = StaticModuleResolver::default();
    static_resolver.insert("archetect", module.clone());
    static_resolver.insert("archetect::archetype", archetype_module);
    static_resolver.insert("archetect::runtime", runtime_module);
    resolver.push(static_resolver);

    let file_module_resolver = FileModuleResolver::new_with_path_and_extension(
        archetype.directory().modules_directory(),
        "rhai",
    );
    resolver.push(file_module_resolver);

    engine.set_module_resolver(resolver);
    engine.register_static_module("archetect", module.into());
}

fn archetype_module(archetype: Archetype) -> Module {
    let mut module = Module::new();

    let at = archetype.clone();
    module.set_native_fn("description", move|| Ok(Dynamic::from(at.directory().manifest().description().to_string())));
    let at = archetype.clone();
    module.set_native_fn(
        "current_script",
        move |call: NativeCallContext| {
            current_script(call, at.clone())
        },
    );
    let at = archetype.clone();
    module.set_native_fn("directory", move||Ok(Dynamic::from(at.directory().root().to_string())));
    let at = archetype.clone();
    module.set_native_fn("authors", move|| {
        let authors = at.directory().manifest().authors()
            .iter()
            .map(|a|Dynamic::from(a.to_owned()))
            .collect::<Vec<Dynamic>>();
        Ok(authors)
    });

    module
}

fn runtime_module(runtime_context: RuntimeContext) -> Module {
    let mut module = Module::new();
    let rt = runtime_context.clone();
    module.set_native_fn("is_offline", move || Ok(rt.offline()));
    let rt = runtime_context.clone();
    module.set_native_fn("is_headless", move || Ok(rt.headless()));
    let rt = runtime_context.clone();
    module.set_native_fn("locals_enabled", move|| Ok(rt.locals().enabled()));
    module
}

fn current_script(call: NativeCallContext, archetype: Archetype) -> Result<Dynamic, Box<EvalAltResult>> {
    let source = call.global_runtime_state().source()
        .map(|f| f.to_owned())
        .map(|f| {
            if !f.ends_with(".rhai") {
                format!("{}/{}.rhai", archetype.directory().modules_directory().to_string(), f)
            } else {
                f
            }
        })
        .unwrap_or("<unknown>".to_owned())
        .to_owned()
        .into()
        ;
    Ok(source)
}


