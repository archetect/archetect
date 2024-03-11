use camino::Utf8PathBuf;
use rhai::plugin::*;
use rhai::{Engine, EvalAltResult, Map, Module};

use archetect_minijinja::Environment;

use crate::archetype::archetype::{render_directory, Archetype, OverwritePolicy};
use crate::archetype::render_context::RenderContext;
use crate::Archetect;
use crate::errors::{ArchetypeScriptError, ArchetypeScriptErrorWrapper};
use crate::script::rhai::modules::path_module::Path;
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
    engine.register_fn("render", render);
    engine.register_fn("render", render_with_settings);
    engine.register_fn("render", render_with_destination);
    engine.register_fn("render", render_with_path);
    engine.register_fn("render", render_with_destination_and_settings);
    engine.register_fn("render", render_with_path_and_settings);
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
}

pub fn render(directory: &mut Directory, context: Map) -> Result<(), Box<EvalAltResult>> {
    let source = directory.archetype.content_directory().join(&directory.path);
    let destination = directory.render_context.destination();
    render_directory(
        &directory.environment,
        &directory.archetect,
        &context,
        source,
        destination,
        OverwritePolicy::Preserve,
    )
        .map_err(|err| Box::new(EvalAltResult::ErrorSystem("Rendering Error".into(), Box::new(err))))
}


pub fn render_with_settings(directory: &mut Directory, context: Map, settings: Map) -> Result<(), Box<EvalAltResult>> {
    let source = directory.archetype.content_directory().join(&directory.path);
    let destination = directory.render_context.destination();
    let overwrite_policy = settings
        .get("if_exists")
        .map(|v| v.clone().try_cast::<OverwritePolicy>())
        .flatten()
        .unwrap_or_default();
    render_directory(
        &directory.environment,
        &directory.archetect,
        &context,
        source,
        destination,
        overwrite_policy,
    )
        .map_err(|err| Box::new(EvalAltResult::ErrorSystem("Rendering Error".into(), Box::new(err))))
}

pub fn render_with_destination(call: NativeCallContext, directory: &mut Directory, destination: Dynamic, context: Map) -> Result<(), Box<EvalAltResult>> {
    let source = directory.archetype.content_directory().join(&directory.path);
    let destination = if destination.is_string() {
        destination.cast::<String>()
    } else if destination.is::<Path>() {
        let mut path = destination.cast::<Path>();
        path.full_path().to_string()
    } else {
        let error = ArchetypeScriptError::RenderDestinationTypeError {
            actual: destination.to_string(),
        };
        return Err(ArchetypeScriptErrorWrapper(&call, error).into());
    };
    let destination = directory
        .render_context
        .destination()
        .join(restrict_path_manipulation(&call, destination.as_str())?);
    render_directory(
        &directory.environment,
        &directory.archetect,
        &context,
        source,
        destination,
        Default::default(),
    )
        .map_err(|err| Box::new(EvalAltResult::ErrorSystem("Rendering Error".into(), Box::new(err))))
}

pub fn render_with_path(call: NativeCallContext, directory: &mut Directory, mut destination: Path, context: Map) -> Result<(), Box<EvalAltResult>> {
    let source = directory.archetype.content_directory().join(&directory.path);
    let destination = directory
        .render_context
        .destination()
        .join(restrict_path_manipulation(&call, destination.path())?);
    render_directory(
        &directory.environment,
        &directory.archetect,
        &context,
        source,
        destination,
        Default::default(),
    )
        .map_err(|err| Box::new(EvalAltResult::ErrorSystem("Rendering Error".into(), Box::new(err))))
}

pub fn render_with_path_and_settings(
    call: NativeCallContext,
    directory: &mut Directory,
    mut destination: Path,
    context: Map,
    settings: Map,
) -> Result<(), Box<EvalAltResult>> {
    let source = directory.archetype.content_directory().join(&directory.path);
    let destination = directory
        .render_context
        .destination()
        .join(restrict_path_manipulation(&call, destination.path())?);
    let overwrite_policy = settings
        .get("if_exists")
        .map(|v| v.clone().try_cast::<OverwritePolicy>())
        .flatten()
        .unwrap_or_default();
    render_directory(
        &directory.environment,
        &directory.archetect,
        &context,
        source,
        destination,
        overwrite_policy,
    )
        .map_err(|err| Box::new(EvalAltResult::ErrorSystem("Rendering Error".into(), Box::new(err))))
}


pub fn render_with_destination_and_settings(
    call: NativeCallContext,
    directory: &mut Directory,
    destination: &str,
    context: Map,
    settings: Map,
) -> Result<(), Box<EvalAltResult>> {
    let source = directory.archetype.content_directory().join(&directory.path);
    let destination = directory
        .render_context
        .destination()
        .join(restrict_path_manipulation(&call, destination)?);
    let overwrite_policy = settings
        .get("if_exists")
        .map(|v| v.clone().try_cast::<OverwritePolicy>())
        .flatten()
        .unwrap_or_default();
    render_directory(
        &directory.environment,
        &directory.archetect,
        &context,
        source,
        destination,
        overwrite_policy,
    )
        .map_err(|err| Box::new(EvalAltResult::ErrorSystem("Rendering Error".into(), Box::new(err))))
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
