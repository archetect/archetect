use crate::config::{Catalog, CatalogEntry, CatalogEntryType};

use read_input::shortcut::input;
use read_input::InputBuild;
use std::collections::{HashMap, HashSet};

#[derive(Debug)]
pub enum CatalogSelectError {
    EmptyCatalog,
}

pub fn select_from_catalog(catalog: &Catalog) -> Result<CatalogEntry, CatalogSelectError> {
    if catalog.entries().len() == 0 {
        return Err(CatalogSelectError::EmptyCatalog);
    }

    let mut catalog = catalog.clone();

    loop {
        if catalog.entries().len() == 0 {
            return Err(CatalogSelectError::EmptyCatalog);
        }

        let mut choices = catalog
            .entries()
            .iter()
            .map(|a| a.clone())
            .enumerate()
            .collect::<HashMap<_, _>>();

        for (id, entry) in catalog.entries().iter().enumerate() {
            println!("{:>2}) {}", id, entry.description());
        }

        let test_values = choices.keys().map(|v| *v).collect::<HashSet<_>>();
        let result = input::<usize>()
            .msg("\nSelect an entry: ")
            .add_test(move |value| test_values.contains(value))
            .err("Please enter the number of a selection from the list.")
            .repeat_msg("Select an entry: ")
            .get();

        let choice = choices.remove(&result).unwrap();
        if choice.entry_type() == &CatalogEntryType::Catalog {
            catalog = Catalog::load(shellexpand::full(choice.source()).unwrap().as_ref()).unwrap();
            continue;
        } else {
            return Ok(choice);
        }
    }
}
