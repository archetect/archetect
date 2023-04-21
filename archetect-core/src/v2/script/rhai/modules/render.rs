use rhai::{Engine, EvalAltResult, Map, NativeCallContext};
use minijinja::Environment;
use crate::v2::archetype::archetype::{Archetype};

pub (crate) fn register(engine: &mut Engine, environment: Environment<'static>) {
    engine.register_fn("render", move |call: NativeCallContext, template: &str, context: Map| {
        let environment = environment.clone();

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
