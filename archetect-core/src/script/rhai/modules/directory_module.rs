use camino::Utf8PathBuf;
use rhai::plugin::*;
use rhai::{Engine, EvalAltResult, Map, Module};

use archetect_minijinja::Environment;

use crate::archetype::archetype::{render_directory, Archetype, OverwritePolicy};
use crate::archetype::render_context::RenderContext;
use crate::Archetect;
use crate::utils::restrict_path_manipulation;

pub(crate) fn register(
    engine: &mut Engine,
    environment: Environment<'static>,
    archetect: Archetect,
    archetype: Archetype,
    render_context: RenderContext,
) {
    engine.register_global_module(exported_module!(module).into());
    let mut module = Module::new();
    let arch = archetype.clone();
    let ctx = render_context.clone();
    let archetect_clone = archetect.clone();
    module.set_native_fn("Directory", move |path: &str| {
        Directory::new(environment.clone(), arch.clone(), archetect_clone.clone(), ctx.clone(), path)
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
    archetect: Archetect,
    render_context: RenderContext,
    path: Utf8PathBuf,
}

impl Directory {
    pub fn new<T: Into<Utf8PathBuf>>(
        environment: Environment<'static>,
        archetype: Archetype,
        archetect: Archetect,
        render_context: RenderContext,
        path: T,
    ) -> Result<Directory, Box<EvalAltResult>> {
        Ok(Directory {
            environment,
            archetect,
            archetype,
            render_context,
            path: path.into(),
        })
    }

    pub fn render(&mut self, context: Map) -> Result<(), Box<EvalAltResult>> {
        let source = self.archetype.content_directory().join(&self.path);
        let destination = self.render_context.destination();
        render_directory(
            &self.environment,
            &self.archetect,
            &context,
            source,
            destination,
            OverwritePolicy::Preserve,
        )
        .map_err(|err| Box::new(EvalAltResult::ErrorSystem("Rendering Error".into(), Box::new(err))))
    }

    pub fn render_with_settings(&mut self, context: Map, settings: Map) -> Result<(), Box<EvalAltResult>> {
        let source = self.archetype.content_directory().join(&self.path);
        let destination = self.render_context.destination();
        let overwrite_policy = settings
            .get("if_exists")
            .map(|v| v.clone().try_cast::<OverwritePolicy>())
            .flatten()
            .unwrap_or_default();
        render_directory(
            &self.environment,
            &self.archetect,
            &context,
            source,
            destination,
            overwrite_policy,
        )
        .map_err(|err| Box::new(EvalAltResult::ErrorSystem("Rendering Error".into(), Box::new(err))))
    }

    pub fn render_with_destination(&mut self, destination: &str, context: Map) -> Result<(), Box<EvalAltResult>> {
        let source = self.archetype.content_directory().join(&self.path);
        let destination = self
            .render_context
            .destination()
            .join(restrict_path_manipulation(destination)?);
        render_directory(
            &self.environment,
            &self.archetect,
            &context,
            source,
            destination,
            Default::default(),
        )
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
            .render_context
            .destination()
            .join(restrict_path_manipulation(destination)?);
        let overwrite_policy = settings
            .get("if_exists")
            .map(|v| v.clone().try_cast::<OverwritePolicy>())
            .flatten()
            .unwrap_or_default();
        render_directory(
            &self.environment,
            &self.archetect,
            &context,
            source,
            destination,
            overwrite_policy,
        )
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
