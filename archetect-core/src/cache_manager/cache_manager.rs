use archetect_inquire::{InquireError, Select};
use std::fmt::{Display, Formatter};

use crate::catalog::{Catalog, CatalogEntry, CatalogItem};
use crate::errors::{ArchetectError, CatalogError};
use crate::Archetect;

pub struct CacheManager {
    archetect: Archetect,
}

impl CacheManager {
    pub fn new(archetect: Archetect) -> CacheManager {
        Self { archetect }
    }

    pub fn manage(&self, catalog: &Catalog) -> Result<(), ArchetectError> {
        let mut catalog = catalog.clone();

        loop {
            let entries = catalog.entries().to_owned();
            if entries.is_empty() {
                return Err(CatalogError::EmptyCatalog.into());
            }

            let choice = self.select_from_entries(entries)?;

            let operations = select_management_operations(&choice);
            match Select::new("Operation:", operations).prompt() {
                Ok(operation) => {
                    match operation {
                        CacheCommand::View => {
                            if let CatalogEntry::Catalog { description: _, source } = choice {
                                catalog = self.archetect.new_catalog(&source)?;
                                continue;
                            }
                        }
                        _ => {
                            choice.execute_cache_command(&self.archetect, operation)?;
                            break;
                        },
                    }
                }
                Err(_) => {
                    break;
                }

            }
        }

        Ok(())
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
                    CatalogEntry::Catalog { .. } => return Ok(item.entry()),
                    CatalogEntry::Archetype { .. } => return Ok(item.entry()),
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

fn select_management_operations(catalog_entry: &CatalogEntry) -> Vec<CacheCommand> {
    let mut operations = vec![];
    operations.push(CacheCommand::Pull);
    operations.push(CacheCommand::Invalidate);
    match catalog_entry {
        CatalogEntry::Group { .. } => {
            unreachable!()
        }
        CatalogEntry::Catalog { .. } => {
            operations.insert(0, CacheCommand::View);
            operations.insert(2, CacheCommand::PullAll);
        },
        CatalogEntry::Archetype { .. } => {}
    }
    operations
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
        CatalogEntry::Archetype { .. } => "📦",
        _ => "📂",
    }
}

#[derive(Copy, Clone)]
pub enum CacheCommand {
    Pull,
    PullAll,
    Invalidate,
    View,
}

impl Display for CacheCommand {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            CacheCommand::View => {
                write!(f, "View Entries")
            }
            CacheCommand::Pull => {
                write!(f, "Pull")
            }
            CacheCommand::PullAll => {
                write!(f, "Pull All")
            }
            CacheCommand::Invalidate => {
                write!(f, "Invalidate")
            }
        }
    }
}
