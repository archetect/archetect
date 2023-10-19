use rhai::{Engine, EvalAltResult, Map, Module};

use crate::errors::ArchetypeError;
use crate::v2::archetype::archetype::Archetype;
use crate::v2::archetype::archetype_context::ArchetypeContext;
use crate::v2::runtime::context::RuntimeContext;
use crate::v2::source::Source;
use crate::Archetect;

pub(crate) fn register(
    engine: &mut Engine,
    parent: Archetype,
    archetype_context: ArchetypeContext,
    runtime_context: RuntimeContext,
) {
    let mut module = Module::new();

    let p = parent.clone();
    let ctx = archetype_context.clone();
    let rc = runtime_context.clone();
    module.set_native_fn("Archetype", move |key: &str| {
        create_archetype(p.clone(), ctx.clone(), rc.clone(), key)
    });
    engine.register_global_module(module.into());

    engine
        .register_type_with_name::<ArchetypeFacade>("Archetype")
        .register_fn("render", ArchetypeFacade::render)
        .register_fn("render", ArchetypeFacade::render_with_settings)
        .register_fn("render", ArchetypeFacade::render_with_destination)
        .register_fn("render", ArchetypeFacade::render_with_destination_and_settings);
}

#[derive(Clone)]
pub struct ArchetypeFacade {
    child: Archetype,
    runtime_context: RuntimeContext,
    archetype_context: ArchetypeContext,
}

impl ArchetypeFacade {
    pub fn render(&mut self, answers: Map) -> Result<(), Box<EvalAltResult>> {
        let destination = self.archetype_context.destination().to_path_buf();
        self.child
            .render_with_destination(destination, self.runtime_context.clone(), answers)?;
        Ok(())
    }

    pub fn render_with_settings(&mut self, answers: Map, settings: Map) -> Result<(), Box<EvalAltResult>> {
        let destination = self.archetype_context.destination().to_path_buf();
        self.child.render_with_destination_and_settings(
            destination,
            self.runtime_context.clone(),
            answers,
            settings,
        )?;
        Ok(())
    }

    pub fn render_with_destination(&mut self, destination: &str, answers: Map) -> Result<(), Box<EvalAltResult>> {
        let destination = self.archetype_context.destination().join(destination);
        self.child
            .render_with_destination(destination, self.runtime_context.clone(), answers)?;

        Ok(())
    }

    pub fn render_with_destination_and_settings(
        &mut self,
        destination: &str,
        answers: Map,
        settings: Map,
    ) -> Result<(), Box<EvalAltResult>> {
        let destination = self.archetype_context.destination().join(destination);
        self.child.render_with_destination_and_settings(
            destination,
            self.runtime_context.clone(),
            answers,
            settings,
        )?;

        Ok(())
    }
}

fn create_archetype(
    parent: Archetype,
    archetype_context: ArchetypeContext,
    runtime_context: RuntimeContext,
    key: &str,
) -> Result<ArchetypeFacade, Box<EvalAltResult>> {
    if let Some(archetypes) = parent.manifest().components() {
        if let Some(path) = archetypes.get(key) {
            let source = Source::detect(&Archetect::build().unwrap(), &runtime_context, path, None).unwrap();
            let child = Archetype::new(&source).unwrap();

            return Ok(ArchetypeFacade {
                child,
                archetype_context,
                runtime_context,
            });
        }
    }

    return Err(Box::new(EvalAltResult::ErrorSystem(
        format!(
            "Archetypes must be registered in archetype.yaml, and '{}' archetype has not been listed there",
            key
        ),
        Box::new(ArchetypeError::ArchetypeKeyNotFound { key: key.to_owned() }),
    )));
}
