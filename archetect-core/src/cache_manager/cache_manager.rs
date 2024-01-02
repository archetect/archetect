use std::fmt::{Display, Formatter};

use archetect_inquire::{InquireError, Select};
use crate::actions::{ArchetectAction, RenderArchetypeInfo};

use crate::Archetect;
use crate::catalog::{Catalog, CatalogItem};
use crate::errors::{ArchetectError, CatalogError};

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
                            if let ArchetectAction::RenderCatalog { description: _, info} = choice {
                                catalog = self.archetect.new_catalog(info.source())?;
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

    pub fn manage_archetype(&self, info: &RenderArchetypeInfo) -> Result<(), ArchetectError> {
        let entry = ArchetectAction::RenderArchetype {
            description: "Manage Archetype".to_string(),
            info: info.clone(),
        };
        let operations = select_management_operations(&entry);
        match Select::new("Operation:", operations).prompt() {
            Ok(operation) => {
                entry.execute_cache_command(&self.archetect, operation)?;
            }
            Err(_) => (),
        }

        Ok(())
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
                    ArchetectAction::RenderGroup {
                        description: _,
                        info,
                    } => {
                        entry_items = info.entries;
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

fn select_management_operations(catalog_entry: &ArchetectAction) -> Vec<CacheCommand> {
    let mut operations = vec![];
    operations.push(CacheCommand::Pull);
    operations.push(CacheCommand::Invalidate);
    match catalog_entry {
        ArchetectAction::RenderGroup { .. } => {
            unreachable!()
        }
        ArchetectAction::RenderCatalog { .. } => {
            operations.insert(0, CacheCommand::View);
            operations.insert(2, CacheCommand::PullAll);
        },
        ArchetectAction::RenderArchetype { .. } => {}
    }
    operations
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
