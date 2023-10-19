use camino::Utf8PathBuf;
use std::collections::{HashMap, HashSet};
use std::ops::Deref;

use clap::ArgMatches;
use log::{error, info};
use read_input::prelude::*;
use rhai::{Dynamic, EvalAltResult, Map};

use archetect_core::v2::catalog::{CatalogEntry, CatalogError, CatalogManifest, CATALOG_FILE_NAME};
use archetect_core::v2::runtime::context::RuntimeContext;
use archetect_core::v2::source::Source;
use archetect_core::Archetect;
use archetect_core::{self, ArchetectError};

pub mod answers;
mod cli;
pub mod vendor;

fn main() {
    let matches = cli::command().get_matches();

    cli::configure(&matches);

    match execute(matches) {
        Ok(()) => (),
        Err(error) => {
            error!("{}", error);
            std::process::exit(-1);
        }
    }
}

fn execute(matches: ArgMatches) -> Result<(), ArchetectError> {
    let mut answers = Map::new();

    if let Some(answer_files) = matches.get_many::<String>("answer-file") {
        for answer_file in answer_files {
            let results = answers::read_answers(answer_file)?;
            answers.extend(results);
        }
    }

    // TODO: Load user answers
    if let Some(answer_matches) = matches.get_many::<String>("answer") {
        let engine = rhai::Engine::new();
        for answer_match in answer_matches {
            let (identifier, value) = archetect_core::config::answers::parse_answer_pair(answer_match).unwrap();
            let result: Result<Dynamic, Box<EvalAltResult>> = engine.eval(&value);
            match result {
                Ok(value) => {
                    answers.insert(identifier.into(), value);
                }
                Err(err) => match err.deref() {
                    EvalAltResult::ErrorVariableNotFound(_, _) => {
                        let result: Result<Dynamic, Box<EvalAltResult>> =
                            engine.eval(format!("\"{}\"", &value).as_str());
                        match result {
                            Ok(value) => {
                                answers.insert(identifier.into(), value);
                            }
                            Err(err) => {
                                return Err(err.into());
                            }
                        }
                    }
                    _ => return Err(err.into()),
                },
            }
        }
    }
    let archetect = Archetect::build()?;
    let runtime_context = create_runtime_context(&matches)?;

    match matches.subcommand() {
        None => {
            catalog(&matches, archetect, runtime_context, answers)?;
        }
        Some(("completions", args)) => cli::completions(args)?,
        Some(("render", args)) => render(args, archetect, runtime_context, answers)?,
        Some(("catalog", args)) => catalog(args, archetect, runtime_context, answers)?,
        _ => {}
    }

    Ok(())
}

fn create_runtime_context(matches: &ArgMatches) -> Result<RuntimeContext, ArchetectError> {
    let mut runtime_context = RuntimeContext::new();
    runtime_context.set_local(matches.get_flag("local"));
    runtime_context.set_headless(matches.get_flag("headless"));
    runtime_context.set_offline(matches.get_flag("offline"));
    if let Some(switches) = matches.get_many::<String>("switches") {
        for switch in switches {
            runtime_context.enable_switch(switch);
        }
    }
    Ok(runtime_context)
}

fn catalog(
    matches: &ArgMatches,
    archetect: Archetect,
    runtime_context: RuntimeContext,
    mut answers: Map,
) -> Result<(), ArchetectError> {
    let default_source = archetect.layout().catalog().as_str().to_owned();
    let source = matches.get_one::<String>("source").unwrap_or_else(|| &default_source);
    let source = Source::detect(&archetect, &runtime_context, source, None)?;

    let mut catalog_file = source.local_path().to_owned();
    if catalog_file.is_dir() {
        catalog_file.push(CATALOG_FILE_NAME);
    }

    if catalog_file.exists() {
        let catalog_source = Source::detect(&archetect, &runtime_context, catalog_file.as_str(), None)?;
        let catalog = CatalogManifest::load(source.clone())?;
        catalog.check_requirements(&runtime_context)?;

        let catalog_entry = select_from_catalog(&archetect, &runtime_context, &catalog, &catalog_source)?;

        match catalog_entry {
            CatalogEntry::Archetype {
                description: _,
                source,
                answers: catalog_answers,
            } => {
                if let Some(catalog_answers) = catalog_answers {
                    for (k, v) in catalog_answers {
                        if !answers.contains_key(&k) {
                            answers.insert(k, v);
                        }
                    }
                }
                let source = Source::detect(&archetect, &runtime_context, &source, None)?;
                let destination = Utf8PathBuf::from(matches.get_one::<String>("destination").unwrap());
                let archetype = archetect_core::v2::archetype::archetype::Archetype::new(&source)?;
                archetype.check_requirements(&runtime_context)?;
                archetype.render_with_destination(destination, runtime_context, answers)?;

                return Ok(());
            }
            _ => unreachable!(),
        }
    } else {
        info!("No catalog file exists at {:?}.", catalog_file);
    }
    Ok(())
}

pub fn render(
    matches: &ArgMatches,
    archetect: Archetect,
    runtime_context: RuntimeContext,
    answers: Map,
) -> Result<(), ArchetectError> {
    let source = matches.get_one::<String>("source").unwrap();
    let source = Source::detect(&archetect, &runtime_context, source, None)?;
    let destination = Utf8PathBuf::from(matches.get_one::<String>("destination").unwrap());

    let archetype = archetect_core::v2::archetype::archetype::Archetype::new(&source)?;

    archetype.check_requirements(&runtime_context)?;
    archetype.render_with_destination(destination, runtime_context, answers)?;
    Ok(())
}

pub fn you_are_sure(message: &str) -> bool {
    input::<bool>()
        .prompting_on_stderr()
        .msg(format!("{} [false]: ", message))
        .default(false)
        .get()
}

pub fn select_from_catalog(
    archetect: &Archetect,
    runtime_context: &RuntimeContext,
    catalog: &CatalogManifest,
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
                let source = Source::detect(archetect, &runtime_context, &source, Some(current_source))?;
                current_source = source.clone();
                catalog = CatalogManifest::load(source)?;
            }
            CatalogEntry::Archetype {
                description: _,
                source: _,
                answers: _,
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
                answers: _,
            } => return Ok(choice),
        }
    }
}
