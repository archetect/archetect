use rhai::module_resolvers::FileModuleResolver;
use rhai::Engine;

use archetect_minijinja::Environment;

use crate::archetype::archetype::Archetype;
use crate::archetype::render_context::RenderContext;
use crate::runtime::context::RuntimeContext;

pub(crate) mod modules;

pub(crate) fn create_engine(
    environment: Environment<'static>,
    archetype: Archetype,
    runtime_context: RuntimeContext,
    render_context: RenderContext,
) -> Engine {
    let mut engine = Engine::new();
    engine.set_module_resolver(FileModuleResolver::new_with_path_and_extension(
        archetype.directory().modules_directory(),
        "rhai",
    ));
    engine.disable_symbol("eval");
    engine.disable_symbol("to_json");

    modules::runtime::register(&mut engine, runtime_context.clone());
    modules::utils::register(&mut engine, runtime_context.clone(), &render_context);
    modules::cases::register(&mut engine);
    modules::exec::register(&mut engine);
    modules::formats::register(&mut engine);
    modules::log::register(&mut engine, runtime_context.clone());
    modules::prompt::register(
        &mut engine,
        render_context.clone(),
        runtime_context.clone(),
    );
    modules::set::register(&mut engine);
    modules::render::register(&mut engine, environment.clone());
    modules::directory::register(
        &mut engine,
        environment.clone(),
        runtime_context.clone(),
        archetype.clone(),
        render_context.clone(),
    );
    modules::archetype::register(
        &mut engine,
        archetype.clone(),
        runtime_context.clone(),
        render_context.clone(),
    );

    engine
}
