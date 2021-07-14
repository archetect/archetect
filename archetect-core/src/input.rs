use crate::config::{Catalog, CatalogEntry, CatalogError};

use crate::source::{Source};
use crate::Archetect;
use crate::vendor::read_input::shortcut::input;
use crate::vendor::read_input::InputBuild;
use std::collections::{HashMap, HashSet};

pub fn you_are_sure(message: &str) -> bool {
    input::<bool>()
        .prompting_on_stderr()
        .msg(format!("{} [false]: ", message))
        .default(false)
        .get()
}

pub fn select_from_catalog(
    archetect: &Archetect,
    catalog: &Catalog,
    current_source: &Source,
) -> Result<CatalogEntry, CatalogError> {
    let mut catalog = catalog.clone();
    let mut current_source = current_source.clone();

    loop {
        if catalog.entries().is_empty() {
            return Err(CatalogError::EmptyCatalog);
        }

        let choice = select_from_entries(archetect, catalog.entries_owned())?;

        match choice {
            CatalogEntry::Catalog { description: _, source } => {
                let source = Source::detect(archetect, &source, Some(current_source))?;
                current_source = source.clone();
                catalog = Catalog::load(source)?;
            }
            CatalogEntry::Archetype {
                description: _,
                source: _,
            } => {
                return Ok(choice);
            }
            CatalogEntry::Group {
                description: _,
                entries: _,
            } => unreachable!(),
        }
    }
}

pub fn select_from_entries(
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

        let test_values = choices.keys().map(|v| *v).collect::<HashSet<_>>();
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
            } => return Ok(choice),
        }
    }
}
