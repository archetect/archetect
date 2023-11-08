use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::Arc;

use read_input::prelude::*;
use rhai::Map;

use crate::Archetect;
use crate::errors::{ArchetectError, CatalogError};
use crate::source::Source;
use crate::v2::archetype::archetype::Archetype;
use crate::v2::catalog::{CatalogEntry, CatalogManifest};
use crate::v2::runtime::context::RuntimeContext;

#[derive(Clone)]
pub struct Catalog {
    pub(crate) inner: Rc<Inner>,
}

pub(crate) struct Inner {
    manifest: CatalogManifest,
}

impl Catalog {
    pub fn load(source: &Source) -> Result<Catalog, CatalogError> {
        let manifest = CatalogManifest::load(source.local_path())?;

        let inner = Rc::new(Inner { manifest });

        let catalog = Catalog { inner };

        Ok(catalog)
    }

    pub fn new(manifest: CatalogManifest) -> Self {
        Catalog {
            inner: Rc::new(Inner { manifest }),
        }
    }

    pub fn check_requirements(&self, runtime_context: &RuntimeContext) -> Result<(), CatalogError> {
        self.inner.manifest.requires().check_requirements(runtime_context)?;
        Ok(())
    }

    pub fn render(
        &self,
        archetect: &Archetect,
        runtime_context: RuntimeContext,
        mut answers: Map,
    ) -> Result<(), ArchetectError> {
        let mut catalog = self.clone();

        loop {
            let entries = catalog.inner.manifest.entries().to_owned();
            if entries.is_empty() {
                return Err(CatalogError::EmptyCatalog.into());
            }

            let choice = self.select_from_entries(archetect, entries)?;

            match choice {
                CatalogEntry::Catalog { description: _, source } => {
                    let source = Source::detect(archetect, &runtime_context, &source, None)?;
                    catalog = Catalog::load(&source)?;
                }
                CatalogEntry::Archetype {
                    description: _,
                    source,
                    answers: catalog_answers,
                } => {
                    if let Some(catalog_answers) = catalog_answers {
                        for (k, v) in catalog_answers {
                            answers.entry(k).or_insert(v);
                        }
                    }
                    let source = Source::detect(&archetect, &runtime_context, &source, None)?;
                    let destination = runtime_context.destination().to_path_buf();
                    let archetype = Archetype::new(&source)?;
                    archetype.check_requirements(&runtime_context)?;
                    archetype.render_with_destination(destination, runtime_context, answers)?;
                    return Ok(());
                }
                CatalogEntry::Group {
                    description: _,
                    entries: _,
                } => unreachable!(),
            }
        }
    }

    pub fn select_from_entries(
        &self,
        _archetect: &Archetect,
        mut entry_items: Vec<CatalogEntry>,
    ) -> Result<CatalogEntry, CatalogError> {
        if entry_items.is_empty() {
            return Err(CatalogError::EmptyGroup);
        }

        loop {
            let mut choices = entry_items
                .iter()
                .enumerate()
                .map(|(id, entry)| (id + 1, entry.clone()))
                .collect::<HashMap<_, _>>();

            for (id, entry) in entry_items.iter().enumerate() {
                eprintln!("{:>2}) {}", id + 1, entry.description());
            }

            let test_values = choices.keys().copied().collect::<HashSet<_>>();
            let result = input::<usize>()
                .prompting_on_stderr()
                .msg("\nSelect an entry: ")
                .add_test(move |value| test_values.contains(value))
                .err("Please enter the number of a selection from the list.")
                .repeat_msg("Select an entry: ")
                .get();

            let choice = choices.remove(&result).unwrap();

            match choice {
                CatalogEntry::Group {
                    description: _,
                    entries,
                } => {
                    entry_items = entries;
                }
                CatalogEntry::Catalog {
                    description: _,
                    source: _,
                } => return Ok(choice),
                CatalogEntry::Archetype {
                    description: _,
                    source: _,
                    answers: _,
                } => return Ok(choice),
            }
        }
    }
}
