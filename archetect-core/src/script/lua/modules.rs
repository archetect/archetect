use mlua::{AnyUserData, Error as LuaError, Lua, Result as LuaResult, Table};

use archetect_templating::Environment;

use crate::archetype::archetype::{render_directory, Archetype, OverwritePolicy};
use crate::archetype::render_context::RenderContext;
use crate::Archetect;

use super::context::Context;

pub fn register_all(
    lua: &Lua,
    archetype: &Archetype,
    archetect: &Archetect,
    render_context: &RenderContext,
    environment: &Environment<'static>,
) -> LuaResult<()> {
    register_context_constructor(lua, archetect, render_context)?;
    super::cases::register_cases(lua)?;
    register_existing_constants(lua)?;
    register_archetype_module(lua, archetype, archetect, render_context, environment)?;
    register_template_module(lua, environment)?;
    register_log(lua, archetect)?;
    register_output(lua, archetect)?;
    Ok(())
}

fn register_context_constructor(
    lua: &Lua,
    archetect: &Archetect,
    render_context: &RenderContext,
) -> LuaResult<()> {
    let archetect = archetect.clone();
    let render_context = render_context.clone();
    let context_table = lua.create_table()?;
    context_table.set(
        "new",
        lua.create_function(move |_, ()| {
            Ok(Context::new(archetect.clone(), render_context.clone()))
        })?,
    )?;
    lua.globals().set("Context", context_table)?;
    Ok(())
}

fn register_log(lua: &Lua, archetect: &Archetect) -> LuaResult<()> {
    let log_table = lua.create_table()?;

    let arc = archetect.clone();
    log_table.set("info", lua.create_function(move |_, msg: String| {
        let _ = arc.request(archetect_api::ScriptMessage::LogInfo(msg));
        Ok(())
    })?)?;

    let arc = archetect.clone();
    log_table.set("debug", lua.create_function(move |_, msg: String| {
        let _ = arc.request(archetect_api::ScriptMessage::LogDebug(msg));
        Ok(())
    })?)?;

    let arc = archetect.clone();
    log_table.set("warn", lua.create_function(move |_, msg: String| {
        let _ = arc.request(archetect_api::ScriptMessage::LogWarn(msg));
        Ok(())
    })?)?;

    let arc = archetect.clone();
    log_table.set("error", lua.create_function(move |_, msg: String| {
        let _ = arc.request(archetect_api::ScriptMessage::LogError(msg));
        Ok(())
    })?)?;

    let arc = archetect.clone();
    log_table.set("trace", lua.create_function(move |_, msg: String| {
        let _ = arc.request(archetect_api::ScriptMessage::LogTrace(msg));
        Ok(())
    })?)?;

    lua.globals().set("log", log_table)?;
    Ok(())
}

fn register_output(lua: &Lua, archetect: &Archetect) -> LuaResult<()> {
    let output_table = lua.create_table()?;

    let arc = archetect.clone();
    output_table.set("print", lua.create_function(move |_, msg: String| {
        let _ = arc.request(archetect_api::ScriptMessage::Print(msg));
        Ok(())
    })?)?;

    let arc = archetect.clone();
    output_table.set("banner", lua.create_function(move |_, msg: String| {
        let _ = arc.request(archetect_api::ScriptMessage::Display(msg));
        Ok(())
    })?)?;

    lua.globals().set("output", output_table)?;
    Ok(())
}

/// UserData for `Archetype("component-name")` — a reference to a registered component.
#[derive(Clone)]
struct ArchetypeRef {
    #[allow(dead_code)]
    name: String,
    child: crate::archetype::archetype::Archetype,
    #[allow(dead_code)]
    render_context: RenderContext,
}

impl mlua::UserData for ArchetypeRef {}

fn register_archetype_module(
    lua: &Lua,
    archetype: &Archetype,
    archetect: &Archetect,
    render_context: &RenderContext,
    environment: &Environment<'static>,
) -> LuaResult<()> {
    let archetype_table = lua.create_table()?;

    // archetype.render(target, ctx, opts?)
    // target is either a string (directory name) or ArchetypeRef (component)
    let arch = archetype.clone();
    let arc = archetect.clone();
    let ctx = render_context.clone();
    let env = environment.clone();

    archetype_table.set(
        "render",
        lua.create_function(
            move |_, (target, context_ud, opts): (mlua::Value, AnyUserData, Option<Table>)| {
                let context = context_ud.borrow::<Context>()?;
                let rhai_map = context.to_rhai_map();

                match target {
                    // String argument → render a content directory
                    mlua::Value::String(dir_name) => {
                        let dir = dir_name.to_string_lossy().to_string();
                        let source = arch.content_directory().join(&dir);
                        let mut destination = ctx.destination().to_owned();

                        if let Some(ref opts) = opts {
                            if let Ok(dest_str) = opts.get::<String>("destination".to_string()) {
                                let dest_str = restrict_path(&dest_str)?;
                                destination = destination.join(dest_str);
                            }
                        }

                        let overwrite_policy = extract_overwrite_policy(&opts);

                        render_directory(&env, &arc, &rhai_map, source, destination, overwrite_policy)
                            .map_err(|e| LuaError::RuntimeError(format!("Render error: {}", e)))
                    }
                    // UserData argument → render a child archetype component
                    mlua::Value::UserData(ud) => {
                        let arch_ref = ud.borrow::<ArchetypeRef>()?;
                        let mut destination = ctx.destination().to_path_buf();

                        // Apply opts: destination, switches, use_defaults, use_defaults_all
                        if let Some(ref opts) = opts {
                            if let Ok(dest_str) = opts.get::<String>("destination".to_string()) {
                                let dest_str = restrict_path(&dest_str)?;
                                destination = destination.join(dest_str);
                            }
                        }

                        let mut child_render_context = RenderContext::new(destination, rhai_map);

                        if let Some(ref opts) = opts {
                            if let Ok(switches) = opts.get::<Vec<String>>("switches".to_string()) {
                                child_render_context.set_switches(switches.into_iter().collect());
                            }
                            if let Ok(defaults) = opts.get::<Vec<String>>("use_defaults".to_string()) {
                                child_render_context.set_use_defaults(defaults.into_iter().collect());
                            }
                            if let Ok(use_defaults_all) = opts.get::<bool>("use_defaults_all".to_string()) {
                                child_render_context.set_use_defaults_all(use_defaults_all);
                            }
                        }

                        let _ = arch_ref.child.render(child_render_context)
                            .map_err(|e| LuaError::RuntimeError(format!("Component render error: {}", e)))?;
                        Ok(())
                    }
                    _ => Err(LuaError::RuntimeError(
                        "archetype.render() expects a directory name (string) or Archetype(\"name\")".to_string()
                    )),
                }
            },
        )?,
    )?;

    let switches = render_context.switches().clone();
    archetype_table.set(
        "switch",
        lua.create_function(move |_, name: String| Ok(switches.contains(&name)))?,
    )?;

    lua.globals().set("archetype", archetype_table)?;

    // Register Archetype("name") constructor as a global
    let parent = archetype.clone();
    let arc2 = archetect.clone();
    let ctx2 = render_context.clone();
    lua.globals().set(
        "Archetype",
        lua.create_function(move |_, name: String| {
            let components = parent.manifest().components().ok_or_else(|| {
                LuaError::RuntimeError("No components defined in archetype.yaml".to_string())
            })?;

            let source = components.get(&name).ok_or_else(|| {
                LuaError::RuntimeError(format!(
                    "Component '{}' not found in archetype.yaml. Available: {:?}",
                    name,
                    components.keys().collect::<Vec<_>>()
                ))
            })?;

            let child = arc2.new_archetype(source).map_err(|e| {
                LuaError::RuntimeError(format!("Failed to load component '{}': {}", name, e))
            })?;

            Ok(ArchetypeRef {
                name,
                child,
                render_context: ctx2.clone(),
            })
        })?,
    )?;

    Ok(())
}

impl mlua::UserData for OverwritePolicy {}

fn register_existing_constants(lua: &Lua) -> LuaResult<()> {
    let table = lua.create_table()?;
    table.set("Overwrite", OverwritePolicy::Overwrite)?;
    table.set("Preserve", OverwritePolicy::Preserve)?;
    table.set("Prompt", OverwritePolicy::Prompt)?;
    lua.globals().set("Existing", table)?;
    Ok(())
}

fn extract_overwrite_policy(opts: &Option<Table>) -> OverwritePolicy {
    if let Some(ref opts) = opts {
        if let Ok(mlua::Value::UserData(ud)) = opts.get::<mlua::Value>("if_exists".to_string()) {
            if let Ok(policy) = ud.borrow::<OverwritePolicy>() {
                return *policy;
            }
        }
    }
    OverwritePolicy::Preserve
}

/// Reject paths that attempt directory traversal or home-relative access.
fn restrict_path(path: &str) -> LuaResult<&str> {
    if path.starts_with("~/") || path.starts_with("../") || path.contains("/../") || path.ends_with("/..") {
        return Err(LuaError::RuntimeError(format!(
            "Path manipulation not allowed: '{}'", path
        )));
    }
    Ok(path)
}

fn register_template_module(lua: &Lua, environment: &Environment<'static>) -> LuaResult<()> {
    let template_table = lua.create_table()?;

    let env = environment.clone();
    template_table.set(
        "render",
        lua.create_function(move |_, (tmpl, context_ud): (String, AnyUserData)| {
            let context = context_ud.borrow::<Context>()?;
            let rhai_map = context.to_rhai_map();
            env.render_str(&tmpl, &rhai_map)
                .map_err(|e| LuaError::RuntimeError(format!("Template error: {}", e)))
        })?,
    )?;

    lua.globals().set("template", template_table)?;
    Ok(())
}
