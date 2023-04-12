use rhai::{Engine, EvalAltResult, Map, NativeCallContext};
use crate::v2::archetype::archetype::{Archetype};

pub (crate) fn register(engine: &mut Engine, archetype: Archetype) {
    let a = archetype.clone();
    engine.register_fn("render", move |call: NativeCallContext, template: &str, context: Map| {
        let inner = &a.inner;
        let environment = &inner.environment;

        match environment.render_str(template, context) {
            Ok(rendered) => Ok(rendered),
            Err(err) => {
                let render_error = EvalAltResult::ErrorSystem(format!("Failed to render \"{}\"", template), Box::new(err));
                let function_name = call.fn_name().to_owned();
                let source = format!("{}", template);
                let function_error = EvalAltResult::ErrorInFunctionCall(function_name, source, Box::new(render_error), call.position());
                return Err(Box::new(function_error));
            }
        }
    });
}