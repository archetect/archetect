use crate::v2::runtime::context::RuntimeContext;
use crate::v2::script::rhai::modules::prompt::handle_result;
use crate::ArchetectError;
use inquire::Confirm;
use rhai::{Dynamic, EvalAltResult, Map};

pub fn prompt(
    message: &str,
    runtime_context: &RuntimeContext,
    settings: &Map,
    _key: Option<&str>,
    _answer: Option<&Dynamic>,
) -> Result<bool, Box<EvalAltResult>> {
    let mut prompt = Confirm::new(message);

    if let Some(default_value) = settings.get("defaults_with") {
        let default_value = match default_value.to_string().to_lowercase().as_str() {
            "y" | "yes" | "t" | "true" => true,
            "n" | "no" | "f" | "false" => false,
            _ => false,
        };
        if runtime_context.headless() {
            return Ok(default_value);
        } else {
            prompt.default = Some(default_value);
        }
    }

    if runtime_context.headless() {
        return Err(Box::new(EvalAltResult::ErrorSystem(
            "Headless Mode Error".to_owned(),
            Box::new(ArchetectError::HeadlessNoDefault),
        )));
    }

    if let Some(placeholder) = settings.get("placeholder") {
        prompt.placeholder = Some(placeholder.to_string());
    }

    if let Some(help_message) = settings.get("help") {
        prompt.help_message = Some(help_message.to_string());
    }

    prompt.parser = &|ans| {
        if ans.len() > 5 {
            return Err(());
        }

        let ans = ans.to_lowercase();

        match ans.as_str() {
            "y" | "yes" | "t" | "true" => Ok(true),
            "n" | "no" | "f" | "false" => Ok(false),
            _ => Err(()),
        }
    };

    let result = prompt.prompt();

    handle_result(result)
}
