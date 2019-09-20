use crate::config::{Catalog, CatalogEntry, CatalogEntryType};

use read_input::shortcut::input;
use read_input::InputBuild;
use std::collections::{HashMap, HashSet};
use crate::Archetect;
use crate::util::{Source, SourceError};

#[derive(Debug)]
pub enum CatalogSelectError {
    EmptyCatalog,
    SourceError(SourceError),
    UnsupportedCatalogSource(String),
}

impl From<SourceError> for CatalogSelectError {
    fn from(cause: SourceError) -> Self {
        CatalogSelectError::SourceError(cause)
    }
}

pub fn select_from_catalog(archetect: &Archetect, catalog: &Catalog) -> Result<CatalogEntry, CatalogSelectError> {
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
            let source = Source::detect(archetect, choice.source(), None)?;
            let path = match source {
                Source::RemoteGit { url, .. } => return Err(CatalogSelectError::UnsupportedCatalogSource(url)),
                Source::RemoteHttp { path, .. } => path,
                Source::LocalDirectory { path } => return Err(CatalogSelectError::UnsupportedCatalogSource(path.display().to_string())),
                Source::LocalFile { path } => path,
            };
            catalog = Catalog::load(shellexpand::full(path.to_str().unwrap()).unwrap().as_ref()).unwrap();
            continue;
        } else {
            return Ok(choice);
        }
    }
}
