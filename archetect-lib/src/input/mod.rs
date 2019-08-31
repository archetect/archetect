use crate::config::{ArchetypeInfo, CatalogConfig};
use read_input::shortcut::input;
use read_input::InputBuild;
use std::collections::{HashMap, HashSet};

#[derive(Debug)]
pub enum CatalogSelectError {
    EmptyCatalog,
}

pub fn select_from_catalog(catalog: &CatalogConfig) -> Result<ArchetypeInfo, CatalogSelectError> {
    if catalog.archetypes().len() == 0 {
        return Err(CatalogSelectError::EmptyCatalog);
    }

    let mut choices = catalog
        .archetypes()
        .iter()
        .map(|a| a.clone())
        .enumerate()
        .collect::<HashMap<_, _>>();

    for (item, archetype_info) in catalog.archetypes().iter().enumerate() {
        println!("{} {}", item, archetype_info.description());
    }

    let test_values = choices.keys().map(|v| *v).collect::<HashSet<_>>();
    let result = input::<usize>()
        .msg("Select an contents: ")
        .add_test(move |value| test_values.contains(value))
        .err("Please select from the list")
        .repeat_msg("The selected value does match any from the list.")
        .get();

    return Ok(choices.remove(&result).unwrap());
}
