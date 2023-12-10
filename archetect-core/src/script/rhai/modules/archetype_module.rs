use log::info;
use rhai::{Dynamic, Engine, EvalAltResult, Map, Module};

use crate::archetype::archetype::Archetype;
use crate::archetype::render_context::RenderContext;
use crate::errors::ArchetypeError;
use crate::Archetect;
use crate::utils::restrict_path_manipulation;

pub(crate) fn register(
    engine: &mut Engine,
    parent: Archetype,
    archetect: Archetect,
    render_context: RenderContext,
) {
    let mut module = Module::new();

    let p = parent.clone();
    let ctx = render_context.clone();
    let archetect_clone = archetect.clone();
    module.set_native_fn("Archetype", move |key: &str| {
        create_archetype(p.clone(), ctx.clone(), archetect_clone.clone(), key)
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
    render_context: RenderContext,
}

// TODO: Allow overwrites
impl ArchetypeFacade {
    pub fn render(&mut self, answers: Map) -> Result<Dynamic, Box<EvalAltResult>> {
        let destination = self.render_context.destination().to_path_buf();
        let render_context = RenderContext::new(destination, answers);
        let result = self.child
            .render( render_context)
            .map_err(|err| {
                Box::new(EvalAltResult::ErrorSystem(
                    "Archetype Render Error".to_string(),
                    Box::new(err),
                ))
            })?;
        Ok(result)
    }

    pub fn render_with_settings(&mut self, answers: Map, settings: Map) -> Result<Dynamic, Box<EvalAltResult>> {
        let destination = self.render_context.destination().to_path_buf();
        let render_context = RenderContext::new(destination, answers).with_settings(settings.clone());

        let result = self.child
            .render(render_context)
            .map_err(|err| {
                Box::new(EvalAltResult::ErrorSystem(
                    "Archetype Render Error".to_string(),
                    Box::new(err),
                ))
            })?;
        Ok(result)
    }

    pub fn render_with_destination(&mut self, destination: &str, answers: Map) -> Result<Dynamic, Box<EvalAltResult>> {
        let destination = self
            .render_context
            .destination()
            .join(restrict_path_manipulation(destination)?);
        let render_context = RenderContext::new(destination, answers);
        let result = self.child
            .render(render_context)
            .map_err(|err| {
                Box::new(EvalAltResult::ErrorSystem(
                    "Archetype Render Error".to_string(),
                    Box::new(err),
                ))
            })?;
        Ok(result)
    }

    pub fn render_with_destination_and_settings(
        &mut self,
        destination: &str,
        answers: Map,
        settings: Map,
    ) -> Result<Dynamic, Box<EvalAltResult>> {
        info!("render_with_destination_and_settings: {:?}", answers);
        let destination = self
            .render_context
            .destination()
            .join(restrict_path_manipulation(destination)?);
        let render_context = RenderContext::new(destination, answers).with_settings(settings.clone());
        let result = self.child
            .render(render_context)
            .map_err(|err| {
                Box::new(EvalAltResult::ErrorSystem(
                    "Archetype Render Error".to_string(),
                    Box::new(err),
                ))
            })?;

        Ok(result)
    }
}

fn create_archetype(
    parent: Archetype,
    render_context: RenderContext,
    archetect: Archetect,
    key: &str,
) -> Result<ArchetypeFacade, Box<EvalAltResult>> {
    if let Some(archetypes) = parent.manifest().components() {
        if let Some(path) = archetypes.get(key) {
            // TODO: Handle unwrap
            let child = archetect.new_archetype(path, false).unwrap();

            return Ok(ArchetypeFacade {
                child,
                render_context,
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
