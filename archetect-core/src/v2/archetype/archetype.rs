use std::fs;
use std::fs::File;
use std::io::Write;
use std::rc::Rc;

use camino::{Utf8Path, Utf8PathBuf};
use content_inspector::ContentType;
use log::{debug, trace};
use rhai::{EvalAltResult, Map, Scope};

use minijinja::{Environment};

use crate::config::RuleAction;
use crate::v2::archetype::archetype_context::ArchetypeContext;
use crate::v2::archetype::directory::ArchetypeDirectory;
use crate::v2::archetype::manifest::ArchetypeManifest;
use crate::v2::script::create_environment;
use crate::v2::script::rhai::create_engine;
use crate::v2::source::Source;
use crate::{ArchetypeError, RenderError};

#[derive(Clone)]
pub struct Archetype {
    pub(crate) inner: Rc<Inner>,
}

pub(crate) struct Inner {
    pub environment: Environment<'static>,
    pub directory: ArchetypeDirectory,
}

impl Archetype {
    pub fn new(source: &Source) -> Result<Archetype, ArchetypeError> {
        let directory = ArchetypeDirectory::new(source.clone())?;

        let environment = create_environment();

        let inner = Rc::new(Inner {
            environment,
            directory,
        });

        let archetype = Archetype { inner };

        Ok(archetype)
    }

    pub fn manifest(&self) -> &ArchetypeManifest {
        &self.inner.directory.manifest()
    }

    pub fn root(&self) -> &Utf8Path {
        self.inner.directory.root()
    }

    pub fn render(&self, answers: Map) -> Result<(), Box<EvalAltResult>> {
        let archetype_context = ArchetypeContext::new(Utf8PathBuf::from("."), &answers);

        let mut scope = Scope::new();
        scope.push_constant("ANSWERS", answers);

        let engine = create_engine(self.clone(), archetype_context.clone());

        let directory = &self.inner.directory;
        let script_contents = &directory.script_contents().map_err(|err| {
            Box::new(EvalAltResult::ErrorSystem(
                "Error getting script contents".to_owned(),
                Box::new(err),
            ))
        })?;
        engine.run_with_scope(&mut scope, script_contents)?;

        Ok(())
    }

    pub fn render_with_settings(&self, context: Map, _settings: Map) -> Result<(), Box<EvalAltResult>> {
        let archetype_context = ArchetypeContext::new(Utf8PathBuf::from("."), &context);

        let mut scope = Scope::new();
        scope.push_constant("ANSWERS", context);

        let engine = create_engine(self.clone(), archetype_context.clone());

        let directory = &self.inner.directory;
        let script_contents = &directory.script_contents().map_err(|err| {
            Box::new(EvalAltResult::ErrorSystem(
                "Error getting script contents".to_owned(),
                Box::new(err),
            ))
        })?;
        engine.run_with_scope(&mut scope, script_contents)?;

        Ok(())
    }

    pub fn render_with_destination<P: Into<Utf8PathBuf>>(&self, destination: P, answers: Map) -> Result<(), Box<EvalAltResult>> {
        let archetype_context = ArchetypeContext::new(destination.into(), &answers);

        let mut scope = Scope::new();
        scope.push_constant("ANSWERS", answers);

        let engine = create_engine(self.clone(), archetype_context.clone());

        let directory = &self.inner.directory;
        let script_contents = &directory.script_contents().map_err(|err| {
            Box::new(EvalAltResult::ErrorSystem(
                "Error getting script contents".to_owned(),
                Box::new(err),
            ))
        })?;
        engine.run_with_scope(&mut scope, script_contents)?;

        Ok(())
    }

    pub fn render_with_destination_and_settings<P: Into<Utf8PathBuf>>(
        &self,
        destination: P,
        context: Map,
        _settings: Map,
    ) -> Result<(), Box<EvalAltResult>> {
        let archetype_context = ArchetypeContext::new(destination.into(), &context);

        let mut scope = Scope::new();
        scope.push_constant("ANSWERS", context);

        let engine = create_engine(self.clone(), archetype_context.clone());

        let directory = &self.inner.directory;
        let script_contents = &directory.script_contents().map_err(|err| {
            Box::new(EvalAltResult::ErrorSystem(
                "Error getting script contents".to_owned(),
                Box::new(err),
            ))
        })?;
        engine.run_with_scope(&mut scope, script_contents)?;

        Ok(())
    }
}

pub fn render_directory<SRC: Into<Utf8PathBuf>, DEST: Into<Utf8PathBuf>>(
    environment: &Environment<'static>,
    context: &Map,
    source: SRC,
    destination: DEST,
) -> Result<(), RenderError> {
    let source = source.into();
    let destination = destination.into();

    for entry in fs::read_dir(&source)? {
        let entry = entry?;
        let path = Utf8PathBuf::from_path_buf(entry.path()).unwrap();

        if path.is_dir() {
            let destination = render_destination(environment, context, &destination, &path)?;
            fs::create_dir_all(destination.as_path())?;
            render_directory(environment, context, path, destination)?;
        } else if path.is_file() {
            // TODO: avoid duplication of file read
            let contents = fs::read(&path)?;

            let action = match content_inspector::inspect(contents.as_slice()) {
                ContentType::BINARY => RuleAction::COPY,
                _ => RuleAction::RENDER,
            };

            let overwrite = false;

            let destination = render_destination(environment, context, &destination, &path)?;
            match action {
                RuleAction::RENDER => {
                    if !destination.exists() {
                        debug!("Rendering   {:?}", destination);
                        let contents = render_contents(environment, context, &path)?;
                        write_contents(destination, &contents)?;
                    } else if overwrite {
                        debug!("Overwriting {:?}", destination);
                        let contents = render_contents(environment, context, &path)?;
                        write_contents(destination, &contents)?;
                    } else {
                        trace!("Preserving  {:?}", destination);
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
    let mut output = File::create(&destination)?;
    output.write(contents.as_bytes())?;
    Ok(())
}

pub fn copy_contents<S: AsRef<Utf8Path>, D: AsRef<Utf8Path>>(source: S, destination: D) -> Result<(), RenderError> {
    let source = source.as_ref();
    let destination = destination.as_ref();
    fs::copy(source, destination)?;
    Ok(())
}

#[cfg(test)]
mod tests {}
