use std::fmt::{Display, Formatter};
use std::rc::Rc;

use inquire::{InquireError, Select};
use rhai::Map;

use crate::errors::{ArchetectError, CatalogError};
use crate::source::Source;
use crate::archetype::archetype::Archetype;
use crate::catalog::{CatalogEntry, CatalogManifest};
use crate::runtime::context::RuntimeContext;
use crate::Archetect;

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
                    let source = Source::detect(archetect, &runtime_context, &source)?;
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
                    let source = Source::detect(&archetect, &runtime_context, &source)?;
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
            let choices = entry_items
                .iter()
                .enumerate()
                .map(|(id, entry)| create_item(entry_items.len(), id, entry))
                .collect::<Vec<_>>();

            let prompt = Select::new("Catalog Selection:", choices).with_page_size(30);

            match prompt.prompt() {
                Ok(item) => match item.entry {
                    CatalogEntry::Group {
                        description: _,
                        entries,
                    } => {
                        entry_items = entries;
                    }
                    CatalogEntry::Catalog {
                        description: _,
                        source: _,
                    } => return Ok(item.entry()),
                    CatalogEntry::Archetype {
                        description: _,
                        source: _,
                        answers: _,
                    } => return Ok(item.entry()),
                },
                Err(err) => {
                    return match err {
                        InquireError::OperationCanceled => Err(CatalogError::SelectionCancelled),
                        InquireError::OperationInterrupted => Err(CatalogError::SelectionCancelled),
                        err => Err(CatalogError::General(err.to_string())),
                    }
                }
            }
        }
    }
}

fn create_item(item_count: usize, id: usize, entry: &CatalogEntry) -> CatalogItem {
    match item_count {
        1..=99 => CatalogItem::new(format!("{:>02}: {}", id + 1, entry.description()), entry.clone()),
        100..=999 => CatalogItem::new(format!("{:>003}: {}", id + 1, entry.description()), entry.clone()),
        _ => CatalogItem::new(format!("{:>0004}: {}", id + 1, entry.description()), entry.clone()),
    }
}

struct CatalogItem {
    text: String,
    entry: CatalogEntry,
}

impl CatalogItem {
    fn new(text: String, entry: CatalogEntry) -> CatalogItem {
        CatalogItem { text, entry }
    }
    fn entry(self) -> CatalogEntry {
        self.entry
    }
}

impl Display for CatalogItem {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.text)
    }
}
