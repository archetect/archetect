use crate::v2::archetype::archetype::Archetype;
use crate::v2::archetype::archetype_context::ArchetypeContext;
use crate::v2::runtime::context::RuntimeContext;
use minijinja::Environment;
use rhai::module_resolvers::FileModuleResolver;
use rhai::Engine;

pub(crate) mod modules;

pub(crate) fn create_engine(
    environment: Environment<'static>,
    archetype: Archetype,
    archetype_context: ArchetypeContext,
    runtime_context: RuntimeContext,
) -> Engine {
    let mut engine = Engine::new();
    engine.set_module_resolver(
        FileModuleResolver::new_with_path_and_extension(archetype.directory().root().join("modules"), "rhai")
    );
    engine.disable_symbol("eval");
    engine.disable_symbol("to_json");

    modules::utils::register(&mut engine, runtime_context.clone());
    modules::cases::register(&mut engine);
    modules::exec::register(&mut engine);
    modules::formats::register(&mut engine);
    modules::log::register(&mut engine);
    modules::prompt::register(
        &mut engine,
        archetype.clone(),
        archetype_context.clone(),
        runtime_context.clone(),
    );
    modules::set::register(&mut engine);
    modules::render::register(&mut engine, environment.clone());
    modules::directory::register(
        &mut engine,
        environment.clone(),
        archetype.clone(),
        archetype_context.clone(),
    );
    modules::archetype::register(
        &mut engine,
        archetype.clone(),
        archetype_context.clone(),
        runtime_context.clone(),
    );

    engine
}
