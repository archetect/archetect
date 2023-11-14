use crate::script::rhai::modules::prompt::handle_result;
use rhai::EvalAltResult;

pub fn prompt(message: &str) -> Result<String, Box<EvalAltResult>> {
    let prompt = inquire::Editor::new(message);

    let result = prompt.prompt();

    handle_result(result)
}
