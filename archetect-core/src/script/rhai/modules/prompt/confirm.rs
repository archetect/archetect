use crate::errors::ArchetectError;
use crate::runtime::context::RuntimeContext;
use crate::script::rhai::modules::prompt::{get_optional_setting, get_render_config, handle_result};
use inquire::Confirm;
use rhai::{Dynamic, EvalAltResult, Map};

pub fn prompt<K: AsRef<str>>(
    message: &str,
    runtime_context: &RuntimeContext,
    settings: &Map,
    _key: Option<K>,
    _answer: Option<&Dynamic>,
) -> Result<Dynamic, Box<EvalAltResult>> {
    let optional = get_optional_setting(settings);

    let mut prompt = Confirm::new(message)
        .with_render_config(get_render_config())
        ;

    if let Some(default_value) = settings.get("defaults_with") {
        let default_value = match default_value.to_string().to_lowercase().as_str() {
            "y" | "yes" | "t" | "true" => true,
            "n" | "no" | "f" | "false" => false,
            _ => false,
        };
        if runtime_context.headless() {
            return Ok(default_value.into());
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
    } else {
        if optional {
            prompt.help_message = Some("<esc> for None".into());
        }
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

    handle_result(prompt.prompt(), optional)
}
