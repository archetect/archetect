
use rhai::Engine;
use uuid::Uuid;
use archetect_api::CommandRequest;
use crate::runtime::context::RuntimeContext;

pub(crate) fn register(engine: &mut Engine, runtime_context: RuntimeContext) {
    let rt = runtime_context.clone();
    engine.register_fn("display", move | message: &str| {
        rt.request(CommandRequest::EPrint(Some(message.to_string())));
    });

    let rt = runtime_context.clone();
    engine.register_fn("display", move || {
        rt.request(CommandRequest::EPrint(None));
    });

    let rt = runtime_context.clone();
    engine.register_fn("eprint", move | message: &str| {
        rt.request(CommandRequest::EPrint(Some(message.to_string())));
    });

    let rt = runtime_context.clone();
    engine.register_fn("eprint", move || {
        rt.request(CommandRequest::EPrint(None));
    });

    engine.register_fn("uuid", move || Uuid::new_v4().to_string());

    let rt = runtime_context.clone();
    engine.register_fn("switch_enabled", move |switch: &str| {
       rt.switch_enabled(switch)
    });
}
