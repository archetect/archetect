use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use log::{debug, trace};
use crate::actions::Actionable;
use crate::{ArchetectError, RenderError};
use crate::config::RuleAction;
use crate::rendering::RenderContext;

pub enum RenderInstruction {
    Directory { source: PathBuf, destination: PathBuf, context: RenderContext }
}

impl Actionable for RenderInstruction {
    fn execute(&self) -> Result<(), ArchetectError> {
        match self {
            RenderInstruction::Directory { source, destination, context } => {
                for entry in fs::read_dir(&source)? {
                    let entry = entry?;
                    let path = entry.path();

                    let action = context.rules_context().get_source_action(path.as_path());

                    if path.is_dir() {
                        let destination = render_destination(&destination, &path, &context)?;
                        debug!("Rendering   {:?}", &destination);
                        fs::create_dir_all(destination.as_path())?;
                        render_directory(context, path, destination)?;
                    } else if path.is_file() {
                        let destination = render_destination(&destination, &path, &context)?;
                        match action {
                            RuleAction::RENDER => {
                                if !destination.exists() {
                                    debug!("Rendering   {:?}", destination);
                                    let contents = render_contents(&path, &context)?;
                                    write_contents(destination, &contents)?;
                                } else if context.rules_context().overwrite() {
                                    debug!("Overwriting {:?}", destination);
                                    let contents = render_contents(&path, &context)?;
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
            }
        }

        Ok(())
    }
}

pub fn render_directory<SRC: Into<PathBuf>, DEST: Into<PathBuf>>(
    context: &RenderContext,
    source: SRC,
    destination: DEST,
) -> Result<(), RenderError> {
    let source = source.into();
    let destination = destination.into();

    for entry in fs::read_dir(&source)? {
        let entry = entry?;
        let path = entry.path();

        let action = context.rules_context().get_source_action(path.as_path());

        if path.is_dir() {
            let destination = render_destination(&destination, &path, &context)?;
            debug!("Rendering   {:?}", &destination);
            fs::create_dir_all(destination.as_path())?;
            render_directory(context, path, destination)?;
        } else if path.is_file() {
            let destination = render_destination(&destination, &path, &context)?;
            match action {
                RuleAction::RENDER => {
                    if !destination.exists() {
                        debug!("Rendering   {:?}", destination);
                        let contents = render_contents(&path, &context)?;
                        write_contents(destination, &contents)?;
                    } else if context.rules_context().overwrite() {
                        debug!("Overwriting {:?}", destination);
                        let contents = render_contents(&path, &context)?;
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

pub fn render_contents<P: AsRef<Path>>(path: P, context: &RenderContext) -> Result<String, RenderError> {
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
    match context.render_string(&template) {
        Ok(result) => Ok(result),
        Err(error) => {
            match error {
                RenderError::StringRenderError { string: _, source } => {
                    Err(RenderError::FileRenderError {
                        path: path.into(),
                        source,
                    })
                }
                original => Err(original)
            }
        }
    }
}


pub fn write_contents<P: AsRef<Path>>(destination: P, contents: &str) -> Result<(), RenderError> {
    let destination = destination.as_ref();
    let mut output = File::create(&destination)?;
    output.write(contents.as_bytes())?;
    Ok(())
}

pub fn copy_contents<S: AsRef<Path>, D: AsRef<Path>>(source: S, destination: D) -> Result<(), RenderError> {
    let source = source.as_ref();
    let destination = destination.as_ref();
    fs::copy(source, destination)?;
    Ok(())
}

fn render_destination<P: AsRef<Path>, C: AsRef<Path>>(
    parent: P,
    child: C,
    context: &RenderContext,
) -> Result<PathBuf, RenderError> {
    let mut destination = parent.as_ref().to_owned();
    let child = child.as_ref();
    let name = render_path(&child, &context)?;
    destination.push(name);
    Ok(destination)
}

fn render_path<P: AsRef<Path>>(path: P, context: &RenderContext) -> Result<String, RenderError> {
    let path = path.as_ref();
    let filename = path.file_name().unwrap_or(path.as_os_str()).to_str().unwrap();
    match context.render_string(filename) {
        Ok(result) => Ok(result),
        Err(error) => {
            match error {
                RenderError::StringRenderError { string: _, source } => {
                    Err(RenderError::FileRenderError {
                        path: path.into(),
                        source,
                    })
                }
                original => Err(original)
            }
        }
    }
}
