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

    lua.load(&script)
        .set_name(script_path.as_str())
        .exec()
        .map_err(|err| {
            let _ = archetect.request(archetect_api::ScriptMessage::LogError(format!("{}", err)));
            let _ = archetect.request(archetect_api::ScriptMessage::CompleteError(format!("{}", err)));
            ArchetypeError::ScriptAbortError
        })?;

    Ok(Dynamic::UNIT)
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
