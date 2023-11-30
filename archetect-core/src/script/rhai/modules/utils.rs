
use rhai::Engine;
use uuid::Uuid;
use archetect_api::CommandRequest;
use crate::archetype::render_context::RenderContext;
use crate::runtime::context::RuntimeContext;

pub(crate) fn register(engine: &mut Engine, runtime_context: RuntimeContext, render_context: &RenderContext) {
    let rt = runtime_context.clone();
    engine.register_fn("display", move | message: &str| {
        rt.request(CommandRequest::Display(message.to_string()));
    });

    let rt = runtime_context.clone();
    engine.register_fn("display", move || {
        rt.request(CommandRequest::Display("".to_string()));
    });

    let rt = runtime_context.clone();
    engine.on_print(move|message| {
        rt.request(CommandRequest::Print(message.to_string()));
    });

    let rt = runtime_context.clone();
    engine.on_debug(move |s, src, pos| {
        let message = if let Some(src) = src {
            format!("{pos:?} | {s}: {src}")
        } else {
            format!("{pos:?} | {s}")
        };
        rt.request(CommandRequest::Display(message));
    });

    let rt = runtime_context.clone();
    engine.on_print(move|message| {
        rt.request(CommandRequest::Print(message.to_string()));
    });

    engine.register_fn("uuid", move || Uuid::new_v4().to_string());

    let switches = render_context.switches().clone();
    engine.register_fn("switch_enabled", move |switch: &str| {
       switches.contains(switch)
    });
}
