use rhai::{Dynamic, Engine, EvalAltResult, Module, NativeCallContext};
use rhai::module_resolvers::{FileModuleResolver, ModuleResolversCollection, StaticModuleResolver};
use rhai::plugin::*;

use crate::Archetect;
use crate::archetype::archetype::Archetype;
use crate::archetype::render_context::RenderContext;

pub(crate) fn register(engine: &mut Engine, archetect: Archetect, archetype: Archetype, render_context: RenderContext) {
    let mut module = Module::new();

    let archetect_clone = archetect.clone();
    module.set_native_fn("version", move || Ok(archetect_clone.version().to_string()));
    let archetect_clone = archetect.clone();
    module.set_native_fn("version_major", move || Ok(archetect_clone.version().major.to_string()));
    let archetect_clone = archetect.clone();
    module.set_native_fn("version_minor", move || Ok(archetect_clone.version().minor.to_string()));
    let archetect_clone = archetect.clone();
    module.set_native_fn("version_patch", move || Ok(archetect_clone.version().patch.to_string()));

    let mut archetype_module = archetype_module(archetype.clone());
    let rc_clone = render_context.clone();
    archetype_module.set_native_fn("settings", move || {
        let mut settings = rhai::Map::new();
        settings.insert("use_defaults_all".into(), rc_clone.use_defaults_all().into());
        settings.insert("use_defaults".into(), rc_clone.use_defaults_as_array().into());
        settings.insert("switches".into(), rc_clone.switches_as_array().into());
        Ok(settings)
    });
    let rc_clone = render_context.clone();
    archetype_module.set_native_fn("use_defaults", move || Ok(rc_clone.use_defaults_as_array()));
    let rc_clone = render_context.clone();
    archetype_module.set_native_fn("use_defaults_all", move || Ok(rc_clone.use_defaults().clone()));
    let rc_clone = render_context.clone();
    archetype_module.set_native_fn("switches", move || Ok(rc_clone.switches().clone()));
    archetype_module.set_native_fn("answers", move || Ok(render_context.clone().answers().clone()));
    module.set_sub_module("archetype", archetype_module.clone());

    let archetect_clone = archetect.clone();
    let runtime_module = runtime_module(archetect_clone.clone());
    module.set_sub_module("runtime", runtime_module.clone());
    let env_module = exported_module!(env_module);
    module.set_sub_module("env", env_module.clone());

    let mut resolver = ModuleResolversCollection::new();

    let mut static_resolver = StaticModuleResolver::default();
    static_resolver.insert("archetect", module.clone());
    static_resolver.insert("archetect::archetype", archetype_module);
    static_resolver.insert("archetect::env", env_module);
    static_resolver.insert("archetect::runtime", runtime_module);
    resolver.push(static_resolver);

    let file_module_resolver =
        FileModuleResolver::new_with_path_and_extension(archetype.directory().modules_directory(), "rhai");
    resolver.push(file_module_resolver);

    engine.set_module_resolver(resolver);
    engine.register_static_module("archetect", module.into());
}

fn archetype_module(archetype: Archetype) -> Module {
    let mut module = Module::new();

    let at = archetype.clone();
    module.set_native_fn("description", move || {
        Ok(Dynamic::from(at.directory().manifest().description().to_string()))
    });
    let at = archetype.clone();
    module.set_native_fn("current_script", move |call: NativeCallContext| {
        current_script(call, at.clone())
    });
    let at = archetype.clone();
    module.set_native_fn("directory", move || {
        Ok(Dynamic::from(at.directory().root().to_string()))
    });
    let at = archetype.clone();
    module.set_native_fn("authors", move || {
        let authors = at
            .directory()
            .manifest()
            .authors()
            .iter()
            .map(|a| Dynamic::from(a.to_owned()))
            .collect::<Vec<Dynamic>>();
        Ok(authors)
    });

    module
}

fn runtime_module(archetect: Archetect) -> Module {
    let mut module = Module::new();
    let archetect_clone = archetect.clone();
    module.set_native_fn("is_offline", move || Ok(archetect_clone.is_offline()));
    let archetect_clone = archetect.clone();
    module.set_native_fn("is_headless", move || Ok(archetect_clone.is_headless()));
    let archetect_clone = archetect.clone();
    module.set_native_fn("locals_enabled", move || {
        Ok(archetect_clone.configuration().locals().enabled())
    });
    module
}

fn current_script(call: NativeCallContext, archetype: Archetype) -> Result<Dynamic, Box<EvalAltResult>> {
    let source = call
        .global_runtime_state()
        .source()
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
        .into();
    Ok(source)
}

#[export_module]
pub mod env_module {
    pub const OS: &'static str = std::env::consts::OS;
    pub const FAMILY: &'static str = std::env::consts::FAMILY;
    pub const ARCH: &'static str = std::env::consts::ARCH;
    pub const DLL_EXTENSION: &'static str = std::env::consts::DLL_EXTENSION;
    pub const DLL_PREFIX: &'static str = std::env::consts::DLL_PREFIX;
    pub const EXE_EXTENSION: &'static str = std::env::consts::EXE_EXTENSION;
    pub const EXE_SUFFIX: &'static str = std::env::consts::EXE_SUFFIX;

    pub fn is_unix() -> bool {
        std::env::consts::FAMILY.eq("unix")
    }

    pub fn is_windows() -> bool {
        std::env::consts::FAMILY.eq("windows")
    }

    pub fn is_macos() -> bool {
        std::env::consts::OS.eq("macos")
    }
}
