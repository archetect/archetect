use rhai::{Dynamic, EvalAltResult, Map, NativeCallContext};

use archetect_api::{ClientMessage, ListPromptInfo, PromptInfo, ScriptMessage};

use crate::Archetect;
use crate::archetype::render_context::RenderContext;
use crate::errors::{ArchetypeScriptError, ArchetypeScriptErrorWrapper};
use crate::script::rhai::modules::prompt_module::{extract_prompt_info, extract_prompt_items_restrictions};

pub fn prompt<'a, K: AsRef<str> + Clone>(
    call: &NativeCallContext,
    message: &str,
    archetect: &Archetect,
    render_context: &RenderContext,
    settings: &Map,
    key: Option<K>,
    answer: Option<&Dynamic>,
) -> Result<Option<Vec<String>>, Box<EvalAltResult>> {
    let mut prompt_info = ListPromptInfo::new(message, key);

    extract_prompt_info(&mut prompt_info, settings).map_err(|error| ArchetypeScriptErrorWrapper(call, error))?;
    extract_prompt_items_restrictions(&mut prompt_info, settings)
        .map_err(|error| ArchetypeScriptErrorWrapper(call, error))?;

    if let Some(answer) = answer {
        if let Some(answer) = answer.clone().try_cast::<String>() {
            let answers = answer
                .split(',')
                .map(|v| v.trim())
                .map(|v| v.to_owned())
                .collect::<Vec<String>>();
            return Ok(answers.into());
        }

        if let Some(answers) = answer.clone().try_cast::<Vec<Dynamic>>() {
            let answers = answers.iter().map(|v| v.to_string()).collect::<Vec<String>>();
            return Ok(answers.into());
        }

        let requirement = " an Array of Strings or a comma-separated String".to_string();
        let error = ArchetypeScriptError::answer_validation_error(answer.to_string(), &prompt_info, requirement);
        return Err(ArchetypeScriptErrorWrapper(call, error).into());
    }

    if let Some(default) = settings.get("defaults_with") {
        if let Some(defaults) = default.clone().try_cast::<Vec<Dynamic>>() {
            let defaults = defaults.into_iter().map(|v| v.to_string()).collect();
            prompt_info.set_default(Some(defaults));
        } else {
            let error =
                ArchetypeScriptError::default_type_error(default.to_string(), &prompt_info, "an Array of Strings");
            return Err(ArchetypeScriptErrorWrapper(call, error).into());
        }
    }

    if archetect.is_headless()
        || render_context.use_defaults_all()
        || render_context.use_defaults().contains(prompt_info.key().unwrap_or(""))
    {
        if let Some(default) = prompt_info.defaults() {
            return Ok(Some(default));
        } else if prompt_info.optional() {
            return Ok(None);
        } else {
            // TODO: Validate empty list
            return Ok(vec![].into());
        }
    }

    archetect.request(ScriptMessage::PromptForList(prompt_info.clone()))?;

    match archetect.receive()? {
        ClientMessage::Array(answer) => {
            return Ok(Some(answer.into()));
        }
        ClientMessage::None => {
            if !prompt_info.optional() {
                let error = ArchetypeScriptError::answer_not_optional(&prompt_info);
                return Err(ArchetypeScriptErrorWrapper(call, error).into());
            } else {
                return Ok(None);
            }
        }
        ClientMessage::Error(error) => {
            return Err(ArchetypeScriptErrorWrapper(call, ArchetypeScriptError::PromptError(error)).into());
        }
        ClientMessage::Abort => {
            return Err(Box::new(EvalAltResult::ErrorTerminated(Dynamic::UNIT, call.position())));
        }
        response => {
            let error = ArchetypeScriptError::unexpected_prompt_response(&prompt_info, "Array of Strings", response);
            return Err(ArchetypeScriptErrorWrapper(call, error).into());
        }
    };
}
