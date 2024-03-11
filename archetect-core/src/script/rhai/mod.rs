use rhai::module_resolvers::FileModuleResolver;
use rhai::Engine;
use rhai::packages::Package;

use archetect_minijinja::Environment;

use crate::archetype::archetype::Archetype;
use crate::archetype::render_context::RenderContext;
use crate::Archetect;
use crate::script::rhai::modules::rand::RandomPackage;

pub(crate) mod modules;

pub(crate) fn create_engine(
    environment: Environment<'static>,
    archetype: Archetype,
    archetect: Archetect,
    render_context: RenderContext,
) -> Engine {
    let mut engine = Engine::new();
    engine.set_module_resolver(FileModuleResolver::new_with_path_and_extension(
        archetype.directory().modules_directory(),
        "rhai",
    ));
    engine.register_global_module(RandomPackage::new().as_shared_module());
    engine.disable_symbol("eval");
    engine.disable_symbol("to_json");

    modules::archetect_module::register(&mut engine, archetect.clone(), archetype.clone());
    modules::utils_module::register(&mut engine, archetect.clone(), &render_context);
    modules::cases_module::register(&mut engine);
    modules::exec_module::register(&mut engine, archetect.clone(), archetype.clone());
    modules::formats_module::register(&mut engine);
    modules::log_module::register(&mut engine, archetect.clone());
    modules::prompt_module::register(
        &mut engine,
        render_context.clone(),
        archetect.clone(),
    );
    modules::set_module::register(&mut engine);
    modules::render_module::register(&mut engine, environment.clone());
    modules::directory_module::register(
        &mut engine,
        environment.clone(),
        archetect.clone(),
        archetype.clone(),
        render_context.clone(),
    );
    modules::archetype_module::register(
        &mut engine,
        archetype.clone(),
        archetect.clone(),
        render_context.clone(),
    );

    engine
}
