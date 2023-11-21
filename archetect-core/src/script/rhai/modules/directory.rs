use camino::Utf8PathBuf;
use rhai::{Engine, EvalAltResult, Map, Module};
use rhai::plugin::*;

use minijinja::Environment;

use crate::utils::restrict_path_manipulation;
use crate::archetype::archetype::{Archetype, OverwritePolicy, render_directory};
use crate::archetype::archetype_context::ArchetypeContext;
use crate::runtime::context::RuntimeContext;

pub(crate) fn register(
    engine: &mut Engine,
    environment: Environment<'static>,
    runtime_context: RuntimeContext,
    archetype: Archetype,
    archetype_context: ArchetypeContext,
) {
    engine.register_global_module(exported_module!(module).into());
    let mut module = Module::new();
    let arch = archetype.clone();
    let ctx = archetype_context.clone();
    let rc = runtime_context.clone();
    module.set_native_fn("Directory", move |path: &str| {
        Directory::new(environment.clone(), arch.clone(), rc.clone(), ctx.clone(), path)
    });
    engine.register_global_module(module.into());

    engine.register_type_with_name::<Directory>("Directory");
    engine.register_fn("render", Directory::render);
    engine.register_fn("render", Directory::render_with_settings);
    engine.register_fn("render", Directory::render_with_destination);
    engine.register_fn("render", Directory::render_with_destination_and_settings);
}

#[derive(Clone)]
pub struct Directory {
    environment: Environment<'static>,
    archetype: Archetype,
    runtime_context: RuntimeContext,
    archetype_context: ArchetypeContext,
    path: Utf8PathBuf,
}

impl Directory {
    pub fn new<T: Into<Utf8PathBuf>>(
        environment: Environment<'static>,
        archetype: Archetype,
        runtime_context: RuntimeContext,
        archetype_context: ArchetypeContext,
        path: T,
    ) -> Result<Directory, Box<EvalAltResult>> {
        Ok(Directory {
            environment,
            runtime_context,
            archetype,
            archetype_context,
            path: path.into(),
        })
    }

    pub fn render(&mut self, context: Map) -> Result<(), Box<EvalAltResult>> {
        let source = self.archetype.content_directory().join(&self.path);
        let destination = self.archetype_context.destination();
        render_directory(&self.environment, &self.runtime_context, &context, source, destination, OverwritePolicy::Preserve)
            .map_err(|err| Box::new(EvalAltResult::ErrorSystem("Rendering Error".into(), Box::new(err))))
    }

    pub fn render_with_settings(&mut self, context: Map, settings: Map) -> Result<(), Box<EvalAltResult>> {
        let source = self.archetype.content_directory().join(&self.path);
        let destination = self.archetype_context.destination();
        let overwrite_policy = settings.get("if_exists")
            .map(|v| v.clone().try_cast::<OverwritePolicy>())
            .flatten()
            .unwrap_or_default();
        render_directory(&self.environment, &self.runtime_context, &context, source, destination, overwrite_policy)
            .map_err(|err| Box::new(EvalAltResult::ErrorSystem("Rendering Error".into(), Box::new(err))))
    }

    pub fn render_with_destination(&mut self, destination: &str, context: Map) -> Result<(), Box<EvalAltResult>> {
        let source = self.archetype.content_directory().join(&self.path);
        let destination = self
            .archetype_context
            .destination()
            .join(restrict_path_manipulation(destination)?);
        render_directory(&self.environment, &self.runtime_context, &context, source, destination, Default::default())
            .map_err(|err| Box::new(EvalAltResult::ErrorSystem("Rendering Error".into(), Box::new(err))))
    }

    pub fn render_with_destination_and_settings(
        &mut self,
        destination: &str,
        context: Map,
        settings: Map,
    ) -> Result<(), Box<EvalAltResult>> {
        let source = self.archetype.content_directory().join(&self.path);
        let destination = self
            .archetype_context
            .destination()
            .join(restrict_path_manipulation(destination)?);
        let overwrite_policy = settings.get("if_exists")
            .map(|v| v.clone().try_cast::<OverwritePolicy>())
            .flatten()
            .unwrap_or_default();
        render_directory(&self.environment, &self.runtime_context, &context, source, destination, overwrite_policy)
            .map_err(|err| Box::new(EvalAltResult::ErrorSystem("Rendering Error".into(), Box::new(err))))
    }
}

#[allow(non_snake_case)]
#[allow(non_upper_case_globals)]
#[export_module]
pub mod module {
    pub type OverwritePolicy = crate::archetype::archetype::OverwritePolicy;

    pub const Overwrite: OverwritePolicy = OverwritePolicy::Overwrite;
    pub const Preserve: OverwritePolicy = OverwritePolicy::Preserve;
    pub const Prompt: OverwritePolicy = OverwritePolicy::Prompt;
}

