use std::fmt::{Display, Formatter};
use std::rc::Rc;

use archetect_inquire::{InquireError, Select};
use crate::actions::ArchetectAction;

use crate::Archetect;
use crate::archetype::render_context::RenderContext;
use crate::catalog::{CatalogManifest};
use crate::errors::{ArchetectError, CatalogError};
use crate::source::Source;

#[derive(Clone)]
pub struct Catalog {
    archetect: Archetect,
    pub(crate) inner: Rc<Inner>,
}

pub(crate) struct Inner {
    source: Option<Source>,
    manifest: CatalogManifest,
}

impl Catalog {
    pub fn load(archetect: Archetect, source: Source) -> Result<Catalog, CatalogError> {
        let manifest = CatalogManifest::load(source.path()?)?;
        let inner = Rc::new(Inner {
            source: Some(source),
            manifest,
        });
        let catalog = Catalog { archetect, inner };
        Ok(catalog)
    }

    pub fn new(archetect: Archetect, manifest: CatalogManifest) -> Self {
        Catalog {
            archetect,
            inner: Rc::new(Inner { source: None, manifest }),
        }
    }

    pub fn source(&self) -> &Option<Source> {
        &self.inner.source
    }

    pub fn check_requirements(&self) -> Result<(), CatalogError> {
        self.inner.manifest.requires().check_requirements(&self.archetect)?;
        Ok(())
    }

    pub fn entries(&self) -> &[ArchetectAction] {
        self.inner.manifest.entries()
    }

    pub fn render(&self, mut render_context: RenderContext) -> Result<(), ArchetectError> {
        let mut catalog = self.clone();

        loop {
            let entries = catalog.inner.manifest.entries().to_owned();
            if entries.is_empty() {
                return Err(CatalogError::EmptyCatalog.into());
            }

            let choice = self.select_from_entries(entries)?;

            match choice {
                ArchetectAction::RenderCatalog { description: _, info } => {
                    catalog = self.archetect.new_catalog(info.source())?;
                }
                ArchetectAction::RenderArchetype {
                    description: _,
                    info,
                } => {
                    let mut answers = render_context.answers_owned();
                    if let Some(catalog_answers) = info.answers() {
                        for (k, v) in catalog_answers {
                            answers.entry(k.clone()).or_insert(v.clone());
                        }
                    }
                    let archetype = self.archetect.new_archetype(info.source())?;
                    render_context = render_context.with_archetype_info(&info);

                    archetype.check_requirements()?;
                    let _result = archetype.render(render_context)?;
                    return Ok(());
                }
                ArchetectAction::RenderGroup {
                    description: _,
                    info: _,
                } => unreachable!(),
            }
        }
    }

    pub fn select_from_entries(&self, mut entry_items: Vec<ArchetectAction>) -> Result<ArchetectAction, CatalogError> {
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
                    ArchetectAction::RenderGroup { description: _, info } => {
                        entry_items = info.actions_owned();
                    }
                    ArchetectAction::RenderCatalog { .. } => return Ok(item.entry()),
                    ArchetectAction::RenderArchetype { .. } => return Ok(item.entry()),
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

fn create_item(item_count: usize, id: usize, entry: &ArchetectAction) -> CatalogItem {
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

fn item_icon(entry: &ArchetectAction) -> &'static str {
    match entry {
        ArchetectAction::RenderArchetype { .. } => "📦",
        _ => "📂",
    }
}

pub(crate) struct CatalogItem {
    text: String,
    pub(crate) entry: ArchetectAction,
}

impl CatalogItem {
    pub fn new(text: String, entry: ArchetectAction) -> CatalogItem {
        CatalogItem { text, entry }
    }
    pub fn entry(self) -> ArchetectAction {
        self.entry
    }
}

impl Display for CatalogItem {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.text)
    }
}
