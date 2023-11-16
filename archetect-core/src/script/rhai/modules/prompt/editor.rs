use crate::script::rhai::modules::prompt::{get_render_config, handle_result};
use rhai::{Dynamic, EvalAltResult};

pub fn prompt(message: &str) -> Result<Dynamic, Box<EvalAltResult>> {
    let prompt = inquire::Editor::new(message)
        .with_render_config(get_render_config())
        .with_predefined_text("test")
        ;

    handle_result(prompt.prompt(), false)
}
