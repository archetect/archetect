use rhai::Engine;
use crate::v2::archetype::archetype::{Archetype};
use crate::v2::archetype::archetype_context::ArchetypeContext;

pub (crate) mod modules;

pub (crate) fn create_engine(archetype: Archetype, archetype_context: ArchetypeContext) -> Engine {
    let mut engine = Engine::new();
    engine.disable_symbol("eval");
    engine.disable_symbol("to_json");

    modules::utils::register(&mut engine);
    modules::cases::register(&mut engine);
    modules::exec::register(&mut engine);
    modules::formats::register(&mut engine);
    modules::log::register(&mut engine);
    modules::prompt::register(&mut engine, archetype.clone(),archetype_context.clone());
    modules::set::register(&mut engine);
    modules::render::register(&mut engine, archetype.clone());
    modules::directory::register(&mut engine, archetype.clone(), archetype_context.clone());
    modules::archetype::register(&mut engine, archetype.clone(), archetype_context.clone());

    engine
}