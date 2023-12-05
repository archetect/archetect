use std::fmt::{Display, Formatter};
use std::rc::Rc;

use archetect_inquire::{InquireError, Select};

use crate::archetype::render_context::RenderContext;
use crate::catalog::{CatalogEntry, CatalogManifest};
use crate::errors::{ArchetectError, CatalogError};
use crate::runtime::context::RuntimeContext;
use crate::source::Source;

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

    pub fn render(&self, runtime_context: RuntimeContext, render_context: RenderContext) -> Result<(), ArchetectError> {
        let mut catalog = self.clone();

        loop {
            let entries = catalog.inner.manifest.entries().to_owned();
            if entries.is_empty() {
                return Err(CatalogError::EmptyCatalog.into());
            }

            let choice = self.select_from_entries(entries)?;

            match choice {
                CatalogEntry::Catalog { description: _, source } => {
                    catalog = runtime_context.new_catalog(&source)?;
                }
                CatalogEntry::Archetype {
                    description: _,
                    source,
                    answers: catalog_answers,
                } => {
                    let mut answers = render_context.answers_owned();
                    if let Some(catalog_answers) = catalog_answers {
                        for (k, v) in catalog_answers {
                            answers.entry(k).or_insert(v);
                        }
                    }
                    let archetype = runtime_context.new_archetype(&source)?;
                    let destination = render_context.destination().to_path_buf();
                    let render_context = RenderContext::new(destination, answers);

                    archetype.check_requirements(&runtime_context)?;
                    let _result = archetype.render(runtime_context, render_context)?;
                    return Ok(());
                }
                CatalogEntry::Group {
                    description: _,
                    entries: _,
                } => unreachable!(),
            }
        }
    }

    pub fn select_from_entries(&self, mut entry_items: Vec<CatalogEntry>) -> Result<CatalogEntry, CatalogError> {
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
        1..=99 => CatalogItem::new(
            format!("{:>02}: {} {}", id + 1, item_icon(&entry), entry.description()),
            entry.clone(),
        ),
        100..=999 => CatalogItem::new(
            format!("{:>003}: {} {}", id + 1, item_icon(&entry), entry.description()),
            entry.clone(),
        ),
        _ => CatalogItem::new(
            format!("{:>0004}: {} {}", id + 1, item_icon(&entry), entry.description()),
            entry.clone(),
        ),
    }
}

fn item_icon(entry: &CatalogEntry) -> &'static str {
    match entry {
        CatalogEntry::Archetype { .. } => "📄",
        _ => "📂",
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
