use std::fs;

use mlua::Lua;

use archetect_api::ContextValue;

use crate::archetype::archetype::Archetype;
use crate::archetype::render_context::RenderContext;
use crate::errors::ArchetypeError;
use crate::Archetect;

pub(crate) mod cases;
mod context;
mod modules;
mod require_modules;
pub(crate) mod template_engine;

pub(crate) fn execute(
    archetype: &Archetype,
    archetect: &Archetect,
    render_context: &RenderContext,
) -> Result<ContextValue, ArchetypeError> {
    let lua = create_lua(archetype, archetect, render_context)?;

    let script_path = archetype.directory().script()
        .ok_or_else(|| ArchetypeError::ArchetypeConfigMissing)?;
    let script = fs::read_to_string(&script_path).map_err(|err| {
        ArchetypeError::IoError(err)
    })?;

    let result: Result<mlua::Value, mlua::Error> = lua.load(&script)
        .set_name(script_path.as_str())
        .eval();

    match result {
        Ok(value) => {
            // If the script returns a Context, convert its data to a ContextMap
            match value {
                mlua::Value::UserData(ud) => {
                    if let Ok(ctx) = ud.borrow::<context::Context>() {
                        Ok(ContextValue::Map(ctx.to_context_map()))
                    } else {
                        Ok(ContextValue::Nil)
                    }
                }
                _ => Ok(ContextValue::Nil),
            }
        }
        Err(err) => {
            // Check for clean exit() — not an error, just early termination
            if is_clean_exit(&err) {
                return Ok(ContextValue::Nil);
            }

            // User cancelled a prompt (Escape / Ctrl-C) — exit cleanly
            if is_prompt_abort(&err) {
                return Ok(ContextValue::Nil);
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

/// Check if a Lua error is a user-initiated prompt abort (Escape / Ctrl-C).
fn is_prompt_abort(err: &mlua::Error) -> bool {
    match err {
        mlua::Error::RuntimeError(msg) if msg.contains("Prompt aborted") => true,
        mlua::Error::CallbackError { cause, .. } => is_prompt_abort(cause),
        _ => false,
    }
}

fn create_lua(
    archetype: &Archetype,
    archetect: &Archetect,
    render_context: &RenderContext,
) -> Result<Lua, ArchetypeError> {
    let lua = Lua::new();

    // Register pre-loaded globals
    modules::register_all(&lua, archetype, archetect, render_context)
        .map_err(|_| ArchetypeError::ScriptAbortError)?;

    // Register require-based modules
    require_modules::register_require_modules(&lua, archetect, render_context)
        .map_err(|_| ArchetypeError::ScriptAbortError)?;

    // Add archetype's modules directory to Lua package.path
    let modules_dir = archetype.directory().modules_directory();
    if modules_dir.exists() {
        let lua_path_addition = format!("{}/?.lua;{}/?/init.lua", modules_dir, modules_dir);
        lua.load(format!(
            "package.path = '{}' .. ';' .. package.path",
            lua_path_addition.replace('\'', "\\'")
        ))
        .exec()
        .map_err(|_| ArchetypeError::ScriptAbortError)?;
    }

    Ok(lua)
}
