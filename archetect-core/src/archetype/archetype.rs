use std::fs;
use std::fs::File;
use std::io::Write;
use std::sync::Arc;

use camino::{Utf8Path, Utf8PathBuf};
use content_inspector::ContentType;
use log::{debug, trace};
use rhai::{Map, Scope};

use archetect_api::CommandRequest;
use inquire::Confirm;
use minijinja::Environment;

use crate::archetype::archetype_directory::ArchetypeDirectory;
use crate::archetype::archetype_manifest::ArchetypeManifest;
use crate::archetype::render_context::RenderContext;
use crate::errors::{ArchetypeError, RenderError};
use crate::runtime::context::RuntimeContext;
use crate::script::create_environment;
use crate::script::rhai::create_engine;
use crate::source::Source;

#[derive(Clone)]
pub struct Archetype {
    pub(crate) inner: Arc<Inner>,
}

pub(crate) struct Inner {
    pub directory: ArchetypeDirectory,
}

impl Archetype {
    pub fn new(source: &Source) -> Result<Archetype, ArchetypeError> {
        let directory = ArchetypeDirectory::new(source.clone())?;
        let inner = Arc::new(Inner { directory });
        let archetype = Archetype { inner };

        Ok(archetype)
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

    pub fn render(&self, runtime_context: RuntimeContext, render_context: RenderContext) -> Result<(), ArchetypeError> {
        let mut scope = Scope::new();
        scope.push_constant("ANSWERS", render_context.answers_owned());
        scope.push_constant("SWITCHES", render_context.switches_as_array());

        let environment = create_environment(self, runtime_context.clone(), &render_context);
        let engine = create_engine(environment, self.clone(), runtime_context.clone(), render_context);

        match engine.compile_file_with_scope(&mut scope,self.directory().script()?.into_std_path_buf()) {

            Ok(ast) => {
                match engine.run_ast_with_scope(&mut scope, &ast) {
                    Ok(_) => {}
                    Err(error) => {
                        runtime_context.request(CommandRequest::LogError(format!("{}", error)));
                        return Err(ArchetypeError::ScriptAbortError);
                    }
                }
            }
            Err(error) => {
                runtime_context.request(CommandRequest::LogError(format!("{}", error)));
                return Err(ArchetypeError::ScriptAbortError);
            }
        }

        Ok(())
    }

    pub fn check_requirements(&self, runtime_context: &RuntimeContext) -> Result<(), ArchetypeError> {
        self.manifest().requires().check_requirements(runtime_context)?;
        Ok(())
    }
}

pub fn render_directory<SRC: Into<Utf8PathBuf>, DEST: Into<Utf8PathBuf>>(
    environment: &Environment<'static>,
    runtime_context: &RuntimeContext,
    context: &Map,
    source: SRC,
    destination: DEST,
    overwrite_policy: OverwritePolicy,
) -> Result<(), RenderError> {
    let source = source.into();
    let destination = destination.into();

    for entry in fs::read_dir(&source)? {
        let entry = entry?;
        let path = Utf8PathBuf::from_path_buf(entry.path()).unwrap();

        if path.is_dir() {
            let destination = render_destination(environment, context, &destination, &path)?;
            fs::create_dir_all(destination.as_path())?;
            render_directory(
                environment,
                runtime_context,
                context,
                path,
                destination,
                overwrite_policy,
            )?;
        } else if path.is_file() {
            // TODO: avoid duplication of file read
            let contents = fs::read(&path)?;

            let action = match content_inspector::inspect(contents.as_slice()) {
                ContentType::BINARY => RuleAction::COPY,
                _ => RuleAction::RENDER,
            };

            let destination = render_destination(environment, context, &destination, &path)?;
            match action {
                RuleAction::RENDER => {
                    if !destination.exists() {
                        debug!("Rendering   {:?}", destination);
                        let contents = render_contents(environment, context, &path)?;
                        write_contents(destination, &contents)?;
                    } else {
                        match overwrite_policy {
                            OverwritePolicy::Overwrite => {
                                debug!("Overwriting {:?}", destination);
                                let contents = render_contents(environment, context, &path)?;
                                write_contents(destination, &contents)?;
                            }
                            OverwritePolicy::Preserve => {
                                trace!("Preserving  {:?}", destination);
                            }
                            OverwritePolicy::Prompt => {
                                if runtime_context.headless() {
                                    trace!("Preserving  {:?}", destination);
                                } else {
                                    if Confirm::new(format!("Overwrite '{}'?", destination).as_str())
                                        .prompt_skippable()
                                        .unwrap_or_default()
                                        .unwrap_or_default()
                                    {
                                        debug!("Overwriting {:?}", destination);
                                        let contents = render_contents(environment, context, &path)?;
                                        write_contents(destination, &contents)?;
                                    }
                                }
                            }
                        }
                    }
                }
                RuleAction::COPY => {
                    debug!("Copying     {:?}", destination);
                    copy_contents(&path, &destination)?;
                }
                RuleAction::SKIP => {
                    trace!("Skipping    {:?}", destination);
                }
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

pub fn write_contents<P: AsRef<Utf8Path>>(destination: P, contents: &str) -> Result<(), RenderError> {
    let destination = destination.as_ref();
    let mut output = File::create(destination)?;
    let _ = output.write(contents.as_bytes())?;
    Ok(())
}

pub fn copy_contents<S: AsRef<Utf8Path>, D: AsRef<Utf8Path>>(source: S, destination: D) -> Result<(), RenderError> {
    let source = source.as_ref();
    let destination = destination.as_ref();
    fs::copy(source, destination)?;
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

#[cfg(test)]
mod tests {}
