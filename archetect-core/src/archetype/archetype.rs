use std::fs;
use std::sync::Arc;

use camino::{Utf8Path, Utf8PathBuf};
use content_inspector::ContentType;
use rhai::{Dynamic, EvalAltResult, Map, Scope};

use archetect_api::{ClientMessage, ExistingFilePolicy, ScriptMessage, WriteDirectoryInfo, WriteFileInfo};
use archetect_templating::Environment;

use crate::Archetect;
use crate::archetype::archetype_directory::ArchetypeDirectory;
use crate::archetype::archetype_manifest::ArchetypeManifest;
use crate::archetype::archetype_manifest::scripting::ScriptEngine;
use crate::archetype::render_context::RenderContext;
use crate::errors::{ArchetypeError, RenderError};
use crate::script::create_environment;
use crate::script::rhai::create_engine;
use crate::source::Source;

#[derive(Clone)]
pub struct Archetype {
    archetect: Archetect,
    pub(crate) inner: Arc<Inner>,
}

pub(crate) struct Inner {
    source: Option<Source>,
    pub directory: ArchetypeDirectory,
}

impl Archetype {
    pub fn new(archetect: Archetect, source: Source) -> Result<Archetype, ArchetypeError> {
        let directory = ArchetypeDirectory::new(source.path()?)?;
        let inner = Arc::new(Inner { directory, source: Some(source) });
        let archetype = Archetype { archetect, inner };

        Ok(archetype)
    }

    pub fn source(&self) -> &Option<Source> {
        &self.inner.source
    }

    pub fn directory(&self) -> &ArchetypeDirectory {
        &self.inner.directory
    }

    pub fn manifest(&self) -> &ArchetypeManifest {
        self.inner.directory.manifest()
    }

    pub fn root(&self) -> &Utf8Path {
        self.inner.directory.root()
    }

    pub fn content_directory(&self) -> Utf8PathBuf {
        self.root().join(self.manifest().templating().content_directory())
    }

    pub fn template_directory(&self) -> Utf8PathBuf {
        self.root().join(self.manifest().templating().templates_directory())
    }

    pub fn render(&self, render_context: RenderContext) -> Result<Dynamic, ArchetypeError> {
        let environment = create_environment(self, self.archetect.clone(), &render_context);

        let (engine, _script_path) = self.resolve_engine_and_script();

        match engine {
            ScriptEngine::Lua => {
                crate::script::lua::execute(self, &self.archetect, &render_context, &environment)
            }
            ScriptEngine::Rhai => {
                self.render_rhai(render_context, environment)
            }
        }
    }

    fn resolve_engine_and_script(&self) -> (ScriptEngine, Utf8PathBuf) {
        let config = self.manifest().scripting();

        // If engine is explicitly set, use manifest's main (or default for that engine)
        if config.engine.is_some() {
            return (config.engine(), config.main());
        }

        // If main script is explicitly set, infer engine from extension
        if config.main.is_some() {
            return (config.engine(), config.main());
        }

        // Auto-detect: check which script file exists
        let rhai_path = self.root().join("archetype.rhai");
        if rhai_path.exists() {
            return (ScriptEngine::Rhai, Utf8PathBuf::from("archetype.rhai"));
        }

        let lua_path = self.root().join("archetype.lua");
        if lua_path.exists() {
            return (ScriptEngine::Lua, Utf8PathBuf::from("archetype.lua"));
        }

        // Default to Lua for new archetypes
        (ScriptEngine::Lua, Utf8PathBuf::from("archetype.lua"))
    }

    fn render_rhai(&self, render_context: RenderContext, environment: Environment<'static>) -> Result<Dynamic, ArchetypeError> {
        let mut scope = Scope::new();
        scope.push_constant("ANSWERS", render_context.answers_owned());
        scope.push_constant("SWITCHES", render_context.switches_as_array());
        scope.push_constant("USE_DEFAULTS", render_context.use_defaults_as_array());
        scope.push_constant("USE_DEFAULTS_ALL", render_context.use_defaults_all());

        let engine = create_engine(environment, self.clone(), self.archetect.clone(), render_context);

        match engine.compile_file_with_scope(&mut scope, self.directory().script()?.into_std_path_buf()) {
            Ok(ast) => {
                match engine.eval_ast_with_scope(&mut scope, &ast) {
                    Ok(result) => {
                        Ok(result)
                    }
                    Err(error) => {
                        return if let EvalAltResult::ErrorTerminated(_0, _1) = *error {
                            Err(ArchetypeError::ScriptAbortError)
                        } else {
                            let error_msg = format!("{}", error);
                            let _ = self.archetect.request(ScriptMessage::LogError(error_msg.clone()));
                            let _ = self.archetect.request(ScriptMessage::CompleteError(error_msg));
                            Err(ArchetypeError::ScriptAbortError)
                        };
                    }
                }
            }
            Err(error) => {
                let error_msg = format!("{}", error);
                let _ = self.archetect.request(ScriptMessage::LogError(error_msg.clone()));
                let _ = self.archetect.request(ScriptMessage::CompleteError(error_msg));
                return Err(ArchetypeError::ScriptAbortError);
            }
        }
    }

    pub fn check_requirements(&self) -> Result<(), ArchetypeError> {
        self.manifest().requires().check_requirements(&self.archetect)?;
        Ok(())
    }
}

pub fn render_directory<SRC: Into<Utf8PathBuf>, DEST: Into<Utf8PathBuf>>(
    environment: &Environment<'static>,
    archetect: &Archetect,
    context: &Map,
    source: SRC,
    destination: DEST,
    overwrite_policy: OverwritePolicy,
) -> Result<(), RenderError> {
    let source = source.into();
    let destination = destination.into();

    send_write_directory(archetect, &destination)?;

    for entry in fs::read_dir(&source)
        .map_err(|err| RenderError::DirectoryListError { path: source.to_path_buf(), source: err })? {
        let entry = entry
            .map_err(|err| RenderError::DirectoryReadError { path: source.clone(), source: err })
            ?;
        let path = Utf8PathBuf::from_path_buf(entry.path()).unwrap();

        if path.is_dir() {
            let destination = render_destination(environment, context, &destination, &path)?;
            send_write_directory(archetect, &destination)?;
            render_directory(
                environment,
                archetect,
                context,
                path,
                destination,
                overwrite_policy,
            )?;
        } else if path.is_file() {
            let contents = fs::read(&path)
                .map_err(|err| RenderError::FileReadError { path: path.to_path_buf(), source: err })?;

            let action = match content_inspector::inspect(contents.as_slice()) {
                ContentType::BINARY => RuleAction::COPY,
                _ => RuleAction::RENDER,
            };

            let destination = render_destination(environment, context, &destination, &path)?;
            match action {
                RuleAction::RENDER => {
                    let contents = render_contents(environment, context, &path)?;
                    send_write_file(archetect, &destination, contents.into_bytes(), overwrite_policy)?;
                }
                RuleAction::COPY => {
                    send_write_file(archetect, &destination, contents, overwrite_policy)?;
                }
                RuleAction::SKIP => {}
            }
        }
    }

    Ok(())
}

fn send_write_directory(archetect: &Archetect, path: &Utf8Path) -> Result<(), RenderError> {
    archetect.request(ScriptMessage::WriteDirectory(WriteDirectoryInfo {
        path: path.to_string(),
    }))?;
    match archetect.response()? {
        ClientMessage::Ack => Ok(()),
        ClientMessage::Error(msg) => Err(RenderError::CreateDirectoryError {
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
        ClientMessage::Ack => Ok(()),
        ClientMessage::Error(msg) => Err(RenderError::WriteError {
            path: destination.to_path_buf(),
            source: std::io::Error::new(std::io::ErrorKind::Other, msg),
        }),
        other => Err(RenderError::UnexpectedResponse(format!("{:?}", other))),
    }
}

fn render_destination<P: AsRef<Utf8Path>, C: AsRef<Utf8Path>>(
    environment: &Environment<'static>,
    context: &Map,
    parent: P,
    child: C,
) -> Result<Utf8PathBuf, RenderError> {
    let child = child.as_ref();
    let name = render_path(environment, context, child)?;
    let mut destination = parent.as_ref().to_owned();
    destination.push(name);
    Ok(destination)
}

fn render_path<P: AsRef<Utf8Path>>(
    environment: &Environment<'static>,
    context: &Map,
    path: P,
) -> Result<String, RenderError> {
    let path = path.as_ref();
    let filename = path.file_name().unwrap_or(path.as_str());
    match environment.render_str(filename, context) {
        Ok(result) => Ok(result),
        Err(error) => Err(RenderError::PathRenderError2 {
            path: path.into(),
            source: error,
        }),
    }
}

pub fn render_contents<P: AsRef<Utf8Path>>(
    environment: &Environment<'static>,
    context: &Map,
    path: P,
) -> Result<String, RenderError> {
    let path = path.as_ref();
    let template = match fs::read_to_string(path) {
        Ok(template) => template,
        Err(error) => {
            return Err(RenderError::FileRenderIOError {
                path: path.to_owned(),
                source: error,
            });
        }
    };
    match environment.render_str(&template, context) {
        Ok(result) => Ok(result),
        Err(error) => Err(RenderError::PathRenderError2 {
            path: path.into(),
            source: error,
        }),
    }
}

pub enum RuleAction {
    COPY,
    RENDER,
    SKIP,
}

#[derive(Copy, Clone, Debug)]
pub enum OverwritePolicy {
    Overwrite,
    Preserve,
    Prompt,
}

impl Default for OverwritePolicy {
    fn default() -> Self {
        OverwritePolicy::Preserve
    }
}

impl From<OverwritePolicy> for ExistingFilePolicy {
    fn from(value: OverwritePolicy) -> Self {
        match value {
            OverwritePolicy::Overwrite => ExistingFilePolicy::Overwrite,
            OverwritePolicy::Preserve => ExistingFilePolicy::Preserve,
            OverwritePolicy::Prompt => ExistingFilePolicy::Prompt,
        }
    }
}

#[cfg(test)]
mod tests {}
