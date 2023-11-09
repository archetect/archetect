
use rhai::Engine;
use uuid::Uuid;
use crate::v2::runtime::context::RuntimeContext;

pub (crate) fn register(engine: &mut Engine, runtime_context: RuntimeContext) {
    engine.register_fn("display", | message: &str| {
        eprintln!("{}", message);
    });

    engine.register_fn("display", || {
        eprintln!();
    });

    engine.register_fn("eprint", | message: &str| {
        eprintln!("{}", message);
    });

    engine.register_fn("eprint", || {
        eprintln!();
    });

    engine.register_fn("uuid", || Uuid::new_v4().to_string());

    let rt = runtime_context.clone();
    engine.register_fn("switch_enabled", move |switch: &str| {
       rt.switch_enabled(switch)
    });
}
