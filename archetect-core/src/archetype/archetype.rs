use std::fs;
use std::fs::File;
use std::io::Write;
use std::sync::Arc;

use camino::{Utf8Path, Utf8PathBuf};
use content_inspector::ContentType;
use rhai::{Dynamic, EvalAltResult, Map, Scope};

use archetect_api::{ExistingFilePolicy, ScriptMessage, WriteDirectoryInfo, WriteFileInfo};
use archetect_minijinja::Environment;

use crate::Archetect;
use crate::archetype::archetype_directory::ArchetypeDirectory;
use crate::archetype::archetype_manifest::ArchetypeManifest;
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
        let inner = Arc::new(Inner {
            directory,
            source: Some(source),
        });
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
        let mut scope = Scope::new();
        scope.push_constant("ANSWERS", render_context.answers_owned());
        scope.push_constant("SWITCHES", render_context.switches_as_array());
        scope.push_constant("USE_DEFAULTS", render_context.use_defaults_as_array());
        scope.push_constant("USE_DEFAULTS_ALL", render_context.use_defaults_all());

        let environment = create_environment(self, self.archetect.clone(), &render_context);
        let engine = create_engine(environment, self.clone(), self.archetect.clone(), render_context);

        match engine.compile_file_with_scope(&mut scope, self.directory().script()?.into_std_path_buf()) {
            Ok(ast) => match engine.eval_ast_with_scope(&mut scope, &ast) {
                Ok(result) => Ok(result),
                Err(error) => {
                    return if let EvalAltResult::ErrorTerminated(_0, _1) = *error {
                        Err(ArchetypeError::ScriptAbortError)
                    } else {
                        self.archetect.request(ScriptMessage::LogError(format!("{}", error)));
                        Err(ArchetypeError::ScriptAbortError)
                    };
                }
            },
            Err(error) => {
                self.archetect.request(ScriptMessage::LogError(format!("{}", error)));
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
    archetect.request(ScriptMessage::WriteDirectory(WriteDirectoryInfo {
        path: destination.to_string(),
    }));
    let _response = archetect.receive();

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
            let destination = render_destination(environment, context, &destination, &path)?;
            archetect.request(ScriptMessage::WriteDirectory(WriteDirectoryInfo {
                path: destination.to_string(),
            }));
            let _response = archetect.receive();
            render_directory(environment, archetect, context, path, destination, overwrite_policy)?;
        } else if path.is_file() {
            let contents = fs::read(&path).map_err(|err| RenderError::FileReadError {
                path: path.to_path_buf(),
                source: err,
            })?;

            let action = match content_inspector::inspect(contents.as_slice()) {
                ContentType::BINARY => RuleAction::COPY,
                _ => RuleAction::RENDER,
            };

            let destination = render_destination(environment, context, &destination, &path)?;
            match action {
                RuleAction::RENDER => {
                    let contents = render_contents(environment, context, &path, contents)?;
                    archetect.request(ScriptMessage::WriteFile(WriteFileInfo {
                        destination: destination.to_string(),
                        contents: contents.into_bytes(),
                        existing_file_policy: overwrite_policy.into(),
                    }));
                    let _response = archetect.receive();
                }
                RuleAction::COPY => {
                    archetect.request(ScriptMessage::WriteFile(WriteFileInfo {
                        destination: destination.to_string(),
                        contents,
                        existing_file_policy: overwrite_policy.into(),
                    }));
                    let _response = archetect.receive();
                }
                RuleAction::SKIP => {}
            }
        }
    }

    Ok(())
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
    contents: Vec<u8>,
) -> Result<String, RenderError> {
    let path: Utf8PathBuf = path.as_ref().to_path_buf();
    let template = String::from_utf8(contents).map_err(|error| RenderError::Utf8ReadError { path: path.clone() })?;
    match environment.render_str(&template, context) {
        Ok(result) => Ok(result),
        Err(error) => Err(RenderError::PathRenderError2 { path, source: error }).into(),
    }
}

pub fn write_contents<P: AsRef<Utf8Path>>(destination: P, contents: &str) -> Result<(), RenderError> {
    let destination = destination.as_ref();
    let mut output = File::create(destination).map_err(|err| RenderError::CreateFileError {
        path: destination.to_path_buf(),
        source: err,
    })?;
    let _ = output
        .write(contents.as_bytes())
        .map_err(|err| RenderError::WriteError {
            path: destination.to_path_buf(),
            source: err,
        })?;
    Ok(())
}

pub fn copy_contents<S: AsRef<Utf8Path>, D: AsRef<Utf8Path>>(source: S, destination: D) -> Result<(), RenderError> {
    let source = source.as_ref();
    let destination = destination.as_ref();
    fs::copy(source, destination).map_err(|error| RenderError::CopyError {
        from: source.to_path_buf(),
        to: destination.to_path_buf(),
        source: error,
    })?;
    Ok(())
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
