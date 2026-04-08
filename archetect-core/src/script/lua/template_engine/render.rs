use std::collections::HashMap;
use std::fs;

use camino::{Utf8Path, Utf8PathBuf};
use content_inspector::ContentType;
use mlua::{Function, Lua, Table};

use archetect_api::{ScriptMessage, WriteDirectoryInfo, WriteFileInfo};

use crate::archetype::archetype::OverwritePolicy;
use crate::errors::RenderError;
use crate::Archetect;

use super::TemplateCompiler;

/// Cache for compiled Lua template functions, keyed by file path.
pub struct TemplateCache {
    cache: HashMap<String, String>,
}

impl TemplateCache {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    /// Get or compile a template, returning the Lua source code.
    pub fn get_or_compile(&mut self, path: &Utf8Path) -> Result<&str, RenderError> {
        let key = path.to_string();
        if !self.cache.contains_key(&key) {
            let template_text = fs::read_to_string(path).map_err(|err| {
                RenderError::FileRenderIOError {
                    path: path.to_owned(),
                    source: err,
                }
            })?;
            let compiled = TemplateCompiler::compile(&template_text, path.as_str())
                .map_err(|err| RenderError::LuaTemplateCompileError {
                    path: path.to_owned(),
                    message: err.to_string(),
                })?;
            self.cache.insert(key.clone(), compiled.source);
        }
        Ok(&self.cache[&key])
    }
}

/// Render a template file using the Lua template engine.
pub fn lua_render_contents(
    lua: &Lua,
    path: &Utf8Path,
    ctx_table: &Table,
    filters_table: &Table,
    cache: &mut TemplateCache,
) -> Result<String, RenderError> {
    let lua_source = cache.get_or_compile(path)?;

    let func: Function = lua.load(lua_source).eval().map_err(|err| {
        RenderError::LuaTemplateRuntimeError {
            path: path.to_owned(),
            message: format!("Failed to load compiled template: {}", err),
        }
    })?;

    let result: String = func
        .call::<String>((ctx_table.clone(), filters_table.clone()))
        .map_err(|err| RenderError::LuaTemplateRuntimeError {
            path: path.to_owned(),
            message: format!("{}", err),
        })?;

    Ok(result)
}

/// Render a file/directory name using the Lua template engine.
/// Only compiles if the name contains `{{`.
pub fn lua_render_path(
    lua: &Lua,
    filename: &str,
    ctx_table: &Table,
    filters_table: &Table,
) -> Result<String, RenderError> {
    if !filename.contains("{{") {
        return Ok(filename.to_string());
    }

    let compiled = TemplateCompiler::compile(filename, "<path>")
        .map_err(|err| RenderError::LuaTemplateCompileError {
            path: Utf8PathBuf::from(filename),
            message: err.to_string(),
        })?;

    let func: Function = lua.load(&compiled.source).eval().map_err(|err| {
        RenderError::LuaTemplateRuntimeError {
            path: Utf8PathBuf::from(filename),
            message: format!("Failed to load path template: {}", err),
        }
    })?;

    let result: String = func
        .call::<String>((ctx_table.clone(), filters_table.clone()))
        .map_err(|err| RenderError::LuaTemplateRuntimeError {
            path: Utf8PathBuf::from(filename),
            message: format!("{}", err),
        })?;

    Ok(result)
}

/// Render a directory tree using the Lua template engine.
pub fn lua_render_directory(
    lua: &Lua,
    archetect: &Archetect,
    ctx_table: &Table,
    filters_table: &Table,
    source: Utf8PathBuf,
    destination: Utf8PathBuf,
    overwrite_policy: OverwritePolicy,
    cache: &mut TemplateCache,
) -> Result<(), RenderError> {
    send_write_directory(archetect, &destination)?;

    for entry in fs::read_dir(&source).map_err(|err| RenderError::DirectoryListError {
        path: source.to_path_buf(),
        source: err,
    })? {
        let entry = entry.map_err(|err| RenderError::DirectoryReadError {
            path: source.clone(),
            source: err,
        })?;
        let path = Utf8PathBuf::from_path_buf(entry.path()).unwrap();

        if path.is_dir() {
            let dest = lua_render_destination(lua, ctx_table, filters_table, &destination, &path)?;
            send_write_directory(archetect, &dest)?;
            lua_render_directory(
                lua,
                archetect,
                ctx_table,
                filters_table,
                path,
                dest,
                overwrite_policy,
                cache,
            )?;
        } else if path.is_file() {
            let contents = fs::read(&path).map_err(|err| RenderError::FileReadError {
                path: path.to_path_buf(),
                source: err,
            })?;

            let is_binary = matches!(
                content_inspector::inspect(contents.as_slice()),
                ContentType::BINARY
            );

            let dest = lua_render_destination(lua, ctx_table, filters_table, &destination, &path)?;

            if is_binary {
                send_write_file(archetect, &dest, contents, overwrite_policy)?;
            } else {
                let rendered = lua_render_contents(lua, &path, ctx_table, filters_table, cache)?;
                send_write_file(archetect, &dest, rendered.into_bytes(), overwrite_policy)?;
            }
        }
    }

    Ok(())
}

fn lua_render_destination(
    lua: &Lua,
    ctx_table: &Table,
    filters_table: &Table,
    parent: &Utf8Path,
    child: &Utf8Path,
) -> Result<Utf8PathBuf, RenderError> {
    let filename = child.file_name().unwrap_or(child.as_str());
    let rendered_name = lua_render_path(lua, filename, ctx_table, filters_table)?;
    let mut destination = parent.to_owned();
    destination.push(rendered_name);
    Ok(destination)
}

fn send_write_directory(archetect: &Archetect, path: &Utf8Path) -> Result<(), RenderError> {
    archetect.request(ScriptMessage::WriteDirectory(WriteDirectoryInfo {
        path: path.to_string(),
    }))?;
    match archetect.response()? {
        archetect_api::ClientMessage::Ack => Ok(()),
        archetect_api::ClientMessage::Error(msg) => Err(RenderError::CreateDirectoryError {
            path: path.to_path_buf(),
            source: std::io::Error::new(std::io::ErrorKind::Other, msg),
        }),
        other => Err(RenderError::UnexpectedResponse(format!("{:?}", other))),
    }
}

fn send_write_file(
    archetect: &Archetect,
    destination: &Utf8Path,
    contents: Vec<u8>,
    overwrite_policy: OverwritePolicy,
) -> Result<(), RenderError> {
    archetect.request(ScriptMessage::WriteFile(WriteFileInfo {
        destination: destination.to_string(),
        contents,
        existing_file_policy: overwrite_policy.into(),
    }))?;
    match archetect.response()? {
        archetect_api::ClientMessage::Ack => Ok(()),
        archetect_api::ClientMessage::Error(msg) => Err(RenderError::WriteError {
            path: destination.to_path_buf(),
            source: std::io::Error::new(std::io::ErrorKind::Other, msg),
        }),
        other => Err(RenderError::UnexpectedResponse(format!("{:?}", other))),
    }
}
