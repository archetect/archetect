use std::fs;

use mlua::Lua;
use rhai::Dynamic;

use archetect_templating::Environment;

use crate::archetype::archetype::Archetype;
use crate::archetype::render_context::RenderContext;
use crate::errors::ArchetypeError;
use crate::Archetect;

pub(crate) mod cases;
mod context;
mod modules;
mod require_modules;

pub(crate) fn execute(
    archetype: &Archetype,
    archetect: &Archetect,
    render_context: &RenderContext,
    environment: &Environment<'static>,
) -> Result<Dynamic, ArchetypeError> {
    let lua = create_lua(archetype, archetect, render_context, environment)?;

    let script_path = archetype.directory().script()?;
    let script = fs::read_to_string(&script_path).map_err(|err| {
        ArchetypeError::IoError(err)
    })?;

    let result: Result<mlua::Value, mlua::Error> = lua.load(&script)
        .set_name(script_path.as_str())
        .eval();

    match result {
        Ok(value) => {
            // If the script returns a Context, convert its data to a rhai::Map
            match value {
                mlua::Value::UserData(ud) => {
                    if let Ok(ctx) = ud.borrow::<context::Context>() {
                        Ok(Dynamic::from(ctx.to_rhai_map()))
                    } else {
                        Ok(Dynamic::UNIT)
                    }
                }
                _ => Ok(Dynamic::UNIT),
            }
        }
        Err(err) => {
            // Check for clean exit() — not an error, just early termination
            if is_clean_exit(&err) {
                return Ok(Dynamic::UNIT);
            }

            let _ = archetect.request(archetect_api::ScriptMessage::LogError(format!("{}", err)));
            let _ = archetect.request(archetect_api::ScriptMessage::CompleteError(format!("{}", err)));
            Err(ArchetypeError::ScriptAbortError)
        }
    }
}

/// Check if a Lua error is a clean exit() call (not a real error).
fn is_clean_exit(err: &mlua::Error) -> bool {
    match err {
        mlua::Error::RuntimeError(msg) if msg.contains(modules::EXIT_SENTINEL) => true,
        mlua::Error::CallbackError { cause, .. } => is_clean_exit(cause),
        _ => false,
    }
}

fn create_lua(
    archetype: &Archetype,
    archetect: &Archetect,
    render_context: &RenderContext,
    environment: &Environment<'static>,
) -> Result<Lua, ArchetypeError> {
    let lua = Lua::new();

    // Register pre-loaded globals
    modules::register_all(&lua, archetype, archetect, render_context, environment)
        .map_err(|_| ArchetypeError::ScriptAbortError)?;

    // Register require-based modules
    require_modules::register_require_modules(&lua, archetect, render_context)
        .map_err(|_| ArchetypeError::ScriptAbortError)?;

    Ok(lua)
}
