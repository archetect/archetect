use rhai::Engine;
use uuid::Uuid;

use archetect_api::ScriptMessage;

use crate::Archetect;
use crate::archetype::render_context::RenderContext;

pub(crate) fn register(engine: &mut Engine, archetect: Archetect, render_context: &RenderContext) {
    let archetect_clone = archetect.clone();
    engine.register_fn("display", move |message: &str| {
        let _ = archetect_clone.request(ScriptMessage::Display(message.to_string()));
    });

    let archetect_clone = archetect.clone();
    engine.register_fn("display", move || {
        let _ = archetect_clone.request(ScriptMessage::Display("".to_string()));
    });

    let archetect_clone = archetect.clone();
    engine.on_print(move |message| {
        let _ = archetect_clone.request(ScriptMessage::Print(message.to_string()));
    });

    let archetect_clone = archetect.clone();
    engine.on_debug(move |s, src, pos| {
        let message = if let Some(src) = src {
            format!("{pos:?} | {s}: {src}")
        } else {
            format!("{pos:?} | {s}")
        };
        let _ = archetect_clone.request(ScriptMessage::Display(message));
    });

    let archetect_clone = archetect.clone();
    engine.on_print(move |message| {
        let _ = archetect_clone.request(ScriptMessage::Print(message.to_string()));
    });

    engine.register_fn("uuid", move || Uuid::new_v4().to_string());

    let switches = render_context.switches().clone();
    engine.register_fn("switch_enabled", move |switch: &str| switches.contains(switch));
}
