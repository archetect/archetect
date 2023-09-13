use crate::v2::script::rhai::modules::prompt::handle_result;
use crate::ArchetypeError;
use inquire::InquireError;
use rhai::EvalAltResult;

pub fn prompt(message: &str) -> Result<String, Box<EvalAltResult>> {
    let prompt = inquire::Editor::new(message);

    let result = prompt.prompt();

    handle_result(result)
}
