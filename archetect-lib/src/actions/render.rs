use std::path::{Path};

use linked_hash_map::LinkedHashMap;

use crate::{Archetect, ArchetectError, Archetype};
use crate::actions::{Action, set};
use crate::config::AnswerInfo;
use crate::template_engine::Context;
use crate::rules::RulesContext;
use std::fs;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum RenderAction {
    #[serde(rename = "directory")]
    Directory(DirectoryOptions),
    #[serde(rename = "archetype")]
    Archetype(ArchetypeOptions),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DirectoryOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    destination: Option<String>,
    source: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ArchetypeOptions {
    #[serde(skip_serializing_if = "Option::is_none", rename="answers-include")]
    answers_include: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    answers: Option<LinkedHashMap<String, AnswerInfo>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    destination: Option<String>,
    source: String,
}

impl DirectoryOptions {
    pub fn new<S: Into<String>>(source: S) -> DirectoryOptions {
        DirectoryOptions {
            source: source.into(),
            destination: None,
        }
    }

    pub fn with_destination<D: Into<String>>(mut self, destination: D) -> DirectoryOptions {
        self.destination = Some(destination.into());
        self
    }
}

impl ArchetypeOptions {
    pub fn new<S: Into<String>>(source: S) -> ArchetypeOptions {
        ArchetypeOptions {
            answers_include: None,
            answers: None,
            source: source.into(),
            destination: None,
        }
    }

    pub fn with_destination<D: Into<String>>(mut self, destination: D) -> ArchetypeOptions {
        self.destination = Some(destination.into());
        self
    }
}

impl Action for RenderAction {
    fn execute<D: AsRef<Path>>(&self,
               archetect: &Archetect,
               archetype: &Archetype,
               destination: D,
               rules_context: &mut RulesContext,
               _answers: &LinkedHashMap<String, AnswerInfo>,
               context: &mut Context,
    ) -> Result<(), ArchetectError> {
        match self {
            RenderAction::Directory(options) => {
                let source = archetype.path().join(&options.source);
                let destination = if let Some(dest) = &options.destination {
                    if let Ok(result) = shellexpand::full(dest) {
                        use log::debug;
                        debug!("Archetype ShellExpand Dest: {}", result);
                    }
                    destination.as_ref().join(archetect.render_string(dest, context)?)
                } else {
                    destination.as_ref().to_owned()
                };
                fs::create_dir_all(destination.as_path())?;
                archetect.render_directory(context, source, destination, rules_context)?;
            }

            RenderAction::Archetype(options) => {
                let destination = if let Some(dest) = &options.destination {
                    destination.as_ref().join(archetect.render_string(dest, context)?)
                } else {
                    destination.as_ref().to_owned()
                };
                let archetype = archetect.load_archetype(&options.source, Some(archetype.source().clone()))?;

                let mut scoped_answers = LinkedHashMap::new();

                if let Some(answers_include) = &options.answers_include {
                    for identifier in answers_include {
                        if let Some(value) = context.get(identifier) {
                            if let Some(string) = value.as_str() {
                                scoped_answers.insert(identifier.to_owned(), AnswerInfo::with_value(string).build());
                            }
                        }
                    }
                }

                // Render any variables used in the definition of the AnswerInfo using the current
                // context before sending the answers into the new Archetype, as it will start off
                // with an empty context and unable to satisfy any variables.
                if let Some(answers) = &options.answers {
                    let rendered_answers = set::render_answers(archetect, answers, context)?;
                    for (key, value) in rendered_answers {
                        scoped_answers.insert(key, value);
                    }
                };

                archetype.execute_script(archetect, &destination, &scoped_answers)?;
            }
        }

        Ok(())
    }
}
