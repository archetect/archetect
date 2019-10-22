use crate::config::{AnswerInfo, ArchetypeConfig, ModuleInfo, PatternType, RuleAction, RuleConfig, ArchetypeInfo, TemplateInfo};
use crate::errors::RenderError;
use crate::template_engine::{Context, Tera};
use crate::util::{Source, SourceError};
use crate::Archetect;
use log::{trace, warn};
use read_input::prelude::*;
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

pub struct Archetype {
    tera: Tera,
    config: ArchetypeConfig,
    path: PathBuf,
    modules: Vec<Module>,
}

impl Archetype {
    pub fn from_source(archetect: &Archetect, source: Source, offline: bool) -> Result<Archetype, ArchetypeError> {
        let tera = Tera::default();

        let local_path = source.local_path();

        let config = ArchetypeConfig::load(local_path)?;

        let mut archetype = Archetype {
            tera,
            config,
            path: local_path.to_owned(),
            modules: vec![],
        };

        let mut modules = vec![];

        for module_info in archetype.configuration().modules() {
            match module_info {
                ModuleInfo::Archetype(archetype_info) => {
                    let source = Source::detect(archetect, archetype_info.source(), Some(source.clone()))?;
                    let archetype = Archetype::from_source(archetect, source, offline)?;
                    modules.push(Module::Archetype(archetype, archetype_info.clone()));
                },
                ModuleInfo::TemplateDirectory(template_info) => {
                    modules.push(Module::Template(template_info.clone()));
                }
            }
        }

        for module in modules {
            archetype.modules.push(module);
        }

        Ok(archetype)
    }

    pub fn configuration(&self) -> &ArchetypeConfig {
        &self.config
    }

    fn render_directory<SRC: Into<PathBuf>, DEST: Into<PathBuf>>(
        &self,
        context: Context,
        source: SRC,
        destination: DEST,
    ) -> Result<(), RenderError> {
        let source = source.into();
        let destination = destination.into();

        if !source.is_dir() {
            if self.configuration().modules().is_empty() {
                warn!(
                    "The archetypes's '{}' directory does not exist, and there are no submodules. Nothing to render.",
                    source.display()
                );
            }
            return Ok(());
        }

        'walking: for entry in fs::read_dir(&source)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                let destination = self.render_destination(&destination, &path, &context)?;
                trace!("Generating {:?}", &destination);
                fs::create_dir_all(destination.as_path()).unwrap();
                self.render_directory(context.clone(), path, destination)?;
            } else if path.is_file() {
                match self.match_rules(&path) {
                    Ok(None) => {
                        let destination = self.render_destination(&destination, &path, &context)?;
                        let contents = self.render_contents(&path, &context)?;
                        self.write_contents(&destination, &contents)?;
                    }
                    Ok(Some(rule)) => {
                        let destination = self.render_destination(&destination, &path, &context)?;
                        if let Some(filter) = rule.filter() {
                            warn!("'filter = (true|false)' in [[rules]] are deprecated.  Please use 'action = (\"{:?}\"|\"{:?}\"|\"{:?}\")', instead.", RuleAction::RENDER, RuleAction::COPY, RuleAction::SKIP);
                            if filter {
                                let contents = self.render_contents(&path, &context)?;
                                self.write_contents(&destination, &contents)?;
                            } else {
                                self.copy_contents(&path, &destination)?;
                            };
                        } else {
                            match rule.action() {
                                RuleAction::RENDER => {
                                    self.render_contents(&path, &context)?;
                                }
                                RuleAction::COPY => {
                                    self.copy_contents(&path, &destination)?;
                                }
                                RuleAction::SKIP => {
                                    trace!("Skipping   {:?}", destination);
                                }
                            }
                        }
                    }
                    Err(err) => return Err(err),
                };
            }
        }

        Ok(())
    }

    fn match_rules<P: AsRef<Path>>(&self, path: P) -> Result<Option<RuleConfig>, RenderError> {
        let path = path.as_ref();
        for path_rule in self.configuration().path_rules() {
            if path_rule.pattern_type() == &PatternType::GLOB {
                for pattern in path_rule.patterns() {
                    let matcher = glob::Pattern::new(pattern).unwrap();
                    if matcher.matches_path(&path) {
                        return Ok(Some(path_rule.to_owned()));
                    }
                }
            }
        }
        Ok(None)
    }

    fn render_path<P: AsRef<Path>>(&self, path: P, context: &Context) -> Result<String, RenderError> {
        let path = path.as_ref();
        let path = path.file_name().unwrap_or(path.as_os_str()).to_str().unwrap();
        match self.tera.render_string(path, context.clone()) {
            Ok(result) => Ok(result),
            Err(error) => {
                // TODO: Get a better error message.
                let message = String::new();
                Err(RenderError::PathRenderError {
                    source: path.into(),
                    error,
                    message,
                })
            }
        }
    }

    fn render_destination<P: AsRef<Path>, C: AsRef<Path>>(
        &self,
        parent: P,
        child: C,
        context: &Context,
    ) -> Result<PathBuf, RenderError> {
        let mut destination = parent.as_ref().to_owned();
        let child = child.as_ref();
        let name = self.render_path(&child, &context)?;
        destination.push(name);
        Ok(destination)
    }

    fn render_contents<P: AsRef<Path>>(&self, path: P, context: &Context) -> Result<String, RenderError> {
        let path = path.as_ref();
        let template = match fs::read_to_string(path) {
            Ok(template) => template,
            Err(error) => {
                return Err(RenderError::FileRenderIOError {
                    source: path.to_owned(),
                    error,
                    message: "".to_string(),
                });
            }
        };
        match self.tera.render_string(&template, context.clone()) {
            Ok(result) => Ok(result),
            Err(error) => {
                // TODO: Get a better error message.
                let message = String::new();
                Err(RenderError::FileRenderError {
                    source: path.into(),
                    error,
                    message,
                })
            }
        }
    }

    fn render_string(&self, contents: &str, context: Context) -> Result<String, RenderError> {
        match self.tera.render_string(contents, context) {
            Ok(contents) => Ok(contents),
            Err(error) => Err(RenderError::StringRenderError {
                source: contents.to_owned(),
                error,
                message: "".to_string(),
            }),
        }
    }

    fn write_contents<P: AsRef<Path>>(&self, destination: P, contents: &str) -> Result<(), RenderError> {
        let destination = destination.as_ref();
        trace!("Generating {:?}", destination);
        let mut output = File::create(&destination)?;
        output.write(contents.as_bytes())?;
        Ok(())
    }

    fn copy_contents<S: AsRef<Path>, D: AsRef<Path>>(&self, source: S, destination: D) -> Result<(), RenderError> {
        let source = source.as_ref();
        let destination = destination.as_ref();
        trace!("Copying    {:?}", destination);
        fs::copy(source, destination)?;
        Ok(())
    }

    pub fn render<D: Into<PathBuf>>(&self, destination: D, context: Context) -> Result<(), ArchetypeError> {
        let destination = destination.into();
        fs::create_dir_all(&destination).unwrap();
        let mut seed = Context::new();

        for (identifier, variable_info) in self.configuration().variables() {
            if variable_info.is_inheritable() {
                if let Some(value) = context.get(identifier) {
                    seed.insert_value(identifier, value);
                }
            }
        }

        for module in &self.modules {
            match module {
                Module::Template(template_info) => {
                    self.render_directory(
                        context.clone(),
                        self.path.clone().join(template_info.source()),
                        &destination,
                    )?;
                },
                Module::Archetype(archetype, archetype_info) => {
                    let subdirectory = self.render_path(archetype_info.destination().unwrap_or("."), &context)?;
                    let destination = destination.clone().join(subdirectory);
                    let mut answers = HashMap::new();
                    if let Some(answers_configs) = archetype_info.answers() {
                        for (identifier, answer_info) in answers_configs {
                            if let Some(value) = answer_info.value() {
                                answers.insert(
                                    identifier.to_owned(),
                                    AnswerInfo::with_value(
                                        &self.render_string(value, context.clone())?).build(),
                                );
                            }
                        }
                    };

                    let context = archetype.get_context(&answers, Some(seed.clone()))?;
                    archetype.render(destination, context)?;
                },
            }
        }
        Ok(())
    }

    pub fn get_context(
        &self,
        answers: &HashMap<String, AnswerInfo>,
        seed: Option<Context>,
    ) -> Result<Context, ArchetypeError> {
        let mut context = seed.unwrap_or_else(|| Context::new());

        for (identifier, variable_info) in self.config.variables() {
            // First, if an explicit answer was provided, use that, overriding an existing context
            // value if necessary.
            if let Some(answer) = answers.get(identifier) {
                if let Some(value) = answer.value() {
                    context.insert(
                        identifier,
                        self.tera
                            .render_string(value, context.clone())
                            .unwrap()
                            .as_str(),
                    );
                }
            }

            // If the context already contains a value, it was either inherited or answered, and
            // should therefore not be overwritten
            if context.contains(identifier) {
                continue;
            }

            // Insert a value if one was specified in the archetype's configuration file.
            if let Some(value) = variable_info.value() {
                context.insert(
                    identifier.as_str(),
                    self.tera
                        .render_string(value, context.clone())
                        .unwrap()
                        .as_str(),
                );
                continue;
            }

            // If we've reached this point, we'll need to prompt the user for an answer.

            // Determine if a default can be provided.
            let default = if let Some(answer) = answers.get(identifier) {
                if let Some(default) = answer.default() {
                    Some(self.render_string(default, context.clone())?)
                } else {
                    None
                }
            } else if let Some(default) = variable_info.default() {
                Some(self.render_string(default, context.clone())?)
            } else {
                None
            };

            let mut prompt = if let Some(prompt) = variable_info.prompt() {
                format!("{} ", prompt.trim())
            } else {
                format!("{}: ", identifier)
            };

            if let Some(default) = &default {
                prompt.push_str(format!("[{}] ", default).as_str());
            };

            let input_builder = input::<String>()
                .msg(&prompt)
                .add_test(|value| value.len() > 0)
                .repeat_msg(&prompt)
                .err("Must be at least 1 character.  Please try again.");
            let value = if let Some(default) = &default {
                input_builder.default(default.clone().to_owned()).get()
            } else {
                input_builder.get()
            };

            context.insert(identifier, &value);
        }

        Ok(context)
    }
}

pub enum Module {
    Archetype(Archetype, ArchetypeInfo),
    Template(TemplateInfo),
}

impl Module {
    pub fn archetype(archetype: Archetype, archetype_info: ArchetypeInfo) -> Module {
        Module::Archetype(archetype, archetype_info)
    }

    pub fn template(template_info: TemplateInfo) -> Module {
        Module::Template(template_info)
    }
}

#[derive(Debug)]
pub enum ArchetypeError {
    ArchetypeInvalid,
    InvalidAnswersConfig,
    ArchetypeSaveFailed,
    SourceError(SourceError),
    RenderError(RenderError),
}

impl From<SourceError> for ArchetypeError {
    fn from(cause: SourceError) -> Self {
        ArchetypeError::SourceError(cause)
    }
}

impl From<RenderError> for ArchetypeError {
    fn from(error: RenderError) -> Self {
        ArchetypeError::RenderError(error)
    }
}

#[cfg(test)]
mod tests {
    use glob::Pattern;
    use std::path::Path;

    #[test]
    fn test_glob_full_directory_path() {
        assert!(Pattern::new("*/projects")
            .unwrap()
            .matches_path(Path::new("/home/luser/projects")));
        assert!(Pattern::new("/home/*/projects")
            .unwrap()
            .matches_path(Path::new("/home/luser/projects")));
        assert!(Pattern::new("/home/*/projects*")
            .unwrap()
            .matches_path(Path::new("/home/luser/projects")));
        assert!(Pattern::new("/h*/*/*ects")
            .unwrap()
            .matches_path(Path::new("/home/luser/projects")));
        assert!(Pattern::new("*/{{ name # train_case }}/*")
            .unwrap()
            .matches_path(Path::new("/home/{{ name # train_case }}/projects")));
        assert!(Pattern::new("*/{{ name | train_case }}/*")
            .unwrap()
            .matches_path(Path::new("/home/{{ name | train_case }}/projects")));
    }

    #[test]
    fn test_glob_full_file_path() {
        assert!(Pattern::new("*/projects/*")
            .unwrap()
            .matches_path(Path::new("/home/luser/projects/image.jpg")));
        assert!(Pattern::new("/home/*/projects/*")
            .unwrap()
            .matches_path(Path::new("/home/luser/projects/image.jpg")));
        assert!(Pattern::new("/h*/*/*ects*jpg")
            .unwrap()
            .matches_path(Path::new("/home/luser/projects/image.jpg")));
        assert!(Pattern::new("*.jpg")
            .unwrap()
            .matches_path(Path::new("/home/luser/projects/image.jpg")));
        assert!(Pattern::new("/home/**/*.jpg")
            .unwrap()
            .matches_path(Path::new("/home/luser/projects/image.jpg")));
        assert!(Pattern::new("*/{{ name # train_case }}/*")
            .unwrap()
            .matches_path(Path::new("/home/{{ name # train_case }}/projects")));
        assert!(Pattern::new("*/{{ name | train_case }}/*")
            .unwrap()
            .matches_path(Path::new("/home/{{ name | train_case }}/projects")));
    }
}
