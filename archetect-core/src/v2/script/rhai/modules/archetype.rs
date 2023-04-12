use crate::v2::archetype::archetype::{Archetype};
use crate::v2::source::Source;
use crate::{Archetect, ArchetypeError};
use rhai::{Engine, EvalAltResult, Map, Module};
use crate::v2::archetype::archetype_context::ArchetypeContext;

pub(crate) fn register(engine: &mut Engine, parent: Archetype, archetype_context: ArchetypeContext) {
    let mut module = Module::new();

    let p = parent.clone();
    let ctx = archetype_context.clone();
    module.set_native_fn("Archetype", move |key: &str| {
        create_archetype(p.clone(), ctx.clone(), key)
    });
    engine.register_global_module( module.into());

    engine.register_type_with_name::<ArchetypeFacade>("Archetype")
        .register_fn("render", ArchetypeFacade::render)
        .register_fn("render", ArchetypeFacade::render_with_settings)
        .register_fn("render", ArchetypeFacade::render_with_destination)
        .register_fn("render", ArchetypeFacade::render_with_destination_and_settings)
    ;
}

#[derive(Clone)]
pub struct ArchetypeFacade {
    parent: Archetype,
    child: Archetype,
    archetype_context: ArchetypeContext,
}

impl ArchetypeFacade {
    pub fn render(&mut self, answers: Map) -> Result<(), Box<EvalAltResult>> {
        let destination = self.archetype_context.destination().to_path_buf();
        self.child.render_with_destination(destination, answers)?;

        Ok(())
    }

    pub fn render_with_settings(&mut self, answers: Map, settings: Map) -> Result<(), Box<EvalAltResult>> {
        let destination = self.archetype_context.destination().to_path_buf();
        self.child.render_with_destination_and_settings(destination, answers, settings)?;

        Ok(())
    }

    pub fn render_with_destination(&mut self, destination: &str, answers: Map) -> Result<(), Box<EvalAltResult>> {
        let destination = self.archetype_context.destination().join(destination);
        self.child.render_with_destination(destination, answers)?;

        Ok(())
    }

    pub fn render_with_destination_and_settings(
        &mut self,
        destination: &str,
        answers: Map,
        settings: Map,
    ) -> Result<(), Box<EvalAltResult>> {
        let destination = self.archetype_context.destination().join(destination);
        self.child.render_with_destination_and_settings(destination, answers, settings)?;

        Ok(())
    }
}

fn create_archetype(parent: Archetype, archetype_context: ArchetypeContext, key: &str) -> Result<ArchetypeFacade, Box<EvalAltResult>> {
    if let Some(path) = parent.manifest().compositions().get(key) {
        let source = Source::detect(&Archetect::build().unwrap(), path, None).unwrap();
        let child = Archetype::new(&source).unwrap();

        Ok(ArchetypeFacade {
            parent,
            child,
            archetype_context,
        })
    } else {
        Err(Box::new(EvalAltResult::ErrorSystem(
            "Cannot find archetype".to_owned(),
            Box::new(ArchetypeError::ArchetypeKeyNotFound { key: key.to_owned() }),
        )))
    }
}
