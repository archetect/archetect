use crate::v2::archetype::archetype::{render_directory, Archetype};
use crate::v2::script::create_environment;
use camino::Utf8PathBuf;
use rhai::{Engine, EvalAltResult, Map, Module};
use crate::v2::archetype::archetype_context::ArchetypeContext;

pub(crate) fn register(engine: &mut Engine, archetype: Archetype, archetype_context: ArchetypeContext) {
    let mut module = Module::new();
    let arch = archetype.clone();
    let ctx = archetype_context.clone();
    module.set_native_fn("Directory", move |path: &str| Directory::new(arch.clone(), ctx.clone(), path));
    engine.register_global_module(module.into());

    engine.register_type_with_name::<Directory>("Directory");
    engine.register_fn("render", Directory::render);
    engine.register_fn("render", Directory::render_with_settings);
    engine.register_fn("render", Directory::render_with_destination);
    engine.register_fn("render", Directory::render_with_destination_and_settings);
}

#[derive(Clone)]
pub struct Directory {
    archetype: Archetype,
    archetype_context: ArchetypeContext,
    path: Utf8PathBuf,
}

impl Directory {
    pub fn new<T: Into<Utf8PathBuf>>(archetype: Archetype, archetype_context: ArchetypeContext, path: T) -> Result<Directory, Box<EvalAltResult>> {
        Ok(Directory {
            archetype,
            archetype_context,
            path: path.into(),
        })
    }

    pub fn render(&mut self, context: Map) -> Result<(), Box<EvalAltResult>> {
        let environment = create_environment();
        let source = self.archetype.root().join(&self.path);
        let destination = self.archetype_context.destination();
        render_directory(&environment, &context, source, destination)
            .map_err(|err| Box::new(EvalAltResult::ErrorSystem("Rendering Error".into(), Box::new(err))))
    }

    pub fn render_with_settings(&mut self, context: Map, _settings: Map) -> Result<(), Box<EvalAltResult>> {
        let environment = create_environment();
        let source = self.archetype.root().join(&self.path);
        let destination = self.archetype_context.destination();
        render_directory(&environment, &context, source, destination)
            .map_err(|err| Box::new(EvalAltResult::ErrorSystem("Rendering Error".into(), Box::new(err))))
    }

    pub fn render_with_destination(&mut self, destination: &str, context: Map) -> Result<(), Box<EvalAltResult>> {
        let environment = create_environment();
        let source = self.archetype.root().join(&self.path);
        let destination = self.archetype_context.destination().join(destination);
        render_directory(&environment, &context, source, destination)
            .map_err(|err| Box::new(EvalAltResult::ErrorSystem("Rendering Error".into(), Box::new(err))))
    }

    pub fn render_with_destination_and_settings(
        &mut self,
        destination: &str,
        context: Map,
        _settings: Map,
    ) -> Result<(), Box<EvalAltResult>> {
        let environment = create_environment();
        let source = self.archetype.root().join(&self.path);
        let destination = self.archetype_context.destination().join(destination);
        render_directory(&environment, &context, source, destination)
            .map_err(|err| Box::new(EvalAltResult::ErrorSystem("Rendering Error".into(), Box::new(err))))
    }
}