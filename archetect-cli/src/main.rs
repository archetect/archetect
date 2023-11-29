use std::collections::HashSet;

use camino::Utf8PathBuf;
use clap::ArgMatches;
use log::error;
use rhai::Map;

use archetect_core::{self};
use archetect_core::Archetect;
use archetect_core::archetype::archetype::Archetype;
use archetect_core::catalog::Catalog;
use archetect_core::configuration::Configuration;
use archetect_core::errors::ArchetectError;
use archetect_core::runtime::context::RuntimeContext;
use archetect_core::source::Source;
use archetect_terminal_io::TerminalIoDriver;

use crate::answers::parse_answer_pair;

mod answers;
mod cli;
mod configuration;
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
    let archetect = Archetect::build()?;

    let configuration = configuration::load_user_config(&archetect, &matches)
        .map_err(|err| ArchetectError::GeneralError(err.to_string()))?;

    let mut answers = Map::new();
    // Load answers from merged configuration
    for (identifier, value) in configuration.answers() {
        answers.insert(identifier.clone(), value.clone());
    }

    // Load answers from answer files
    if let Some(answer_files) = matches.get_many::<String>("answer-file") {
        for answer_file in answer_files {
            let results = answers::read_answers(answer_file)?;
            answers.extend(results);
        }
    }

    // Load answers from individual answer arguments
    if let Some(answer_matches) = matches.get_many::<String>("answer") {
        for answer_match in answer_matches {
            let (identifier, value) = parse_answer_pair(answer_match).unwrap();
            if let Ok(value) = value.parse::<i64>() {
                answers.insert(identifier.into(), value.into());
            } else if let Ok(value) = value.parse::<bool>() {
                answers.insert(identifier.into(), value.into());
            } else {
                answers.insert(identifier.into(), value.into());
            }
        }
    }
    let archetect = Archetect::build()?;

    match matches.subcommand() {
        None => {
            default(&matches, &archetect, &configuration, answers)?;
        }
        Some(("completions", args)) => cli::completions(args)?,
        Some(("render", args)) => render(args, archetect, &configuration, answers)?,
        Some(("catalog", args)) => catalog(args, archetect, &configuration, answers)?,
        Some(("config", args)) => config(args, &configuration)?,
        _ => {}
    }

    Ok(())
}

fn create_runtime_context(
    matches: &ArgMatches,
    configuration: &Configuration,
) -> Result<RuntimeContext, ArchetectError> {
    let mut switches = HashSet::new();
    if let Some(answer_switches) = matches.get_many::<String>("switches") {
        for switch in answer_switches {
            switches.insert(switch.to_string());
        }
    }
    let destination = Utf8PathBuf::from(matches.get_one::<String>("destination").unwrap());
    let runtime_context = RuntimeContext::new(configuration, switches, destination, TerminalIoDriver::default());

    Ok(runtime_context)
}

fn config(matches: &ArgMatches, configuration: &Configuration) -> Result<(), ArchetectError> {
    match matches.subcommand() {
        Some(("merged", _args)) => {
            println!("{}", configuration.to_yaml());
        }
        Some(("defaults", _args)) => {
            println!("{}", Configuration::default().to_yaml());
        }
        None => {}
        _ => {}
    }

    Ok(())
}

fn default(
    matches: &ArgMatches,
    archetect: &Archetect,
    configuration: &Configuration,
    answers: Map,
) -> Result<(), ArchetectError> {
    let runtime_context = create_runtime_context(matches, configuration)?;
    let catalog = configuration.catalog();
    catalog.render(archetect, runtime_context, answers)?;
    Ok(())
}

fn catalog(
    matches: &ArgMatches,
    archetect: Archetect,
    configuration: &Configuration,
    answers: Map,
) -> Result<(), ArchetectError> {
    let runtime_context = create_runtime_context(matches, configuration)?;
    let source = matches.get_one::<String>("source").unwrap();
    let source = Source::detect(&archetect, &runtime_context, source)?;

    let catalog = Catalog::load(&source)?;
    catalog.check_requirements(&runtime_context)?;
    catalog.render(&archetect, runtime_context, answers)?;
    Ok(())
}

pub fn render(
    matches: &ArgMatches,
    archetect: Archetect,
    configuration: &Configuration,
    answers: Map,
) -> Result<(), ArchetectError> {
    let runtime_context = create_runtime_context(matches, configuration)?;
    let source = matches.get_one::<String>("source").unwrap();
    let source = Source::detect(&archetect, &runtime_context, source)?;

    let archetype = Archetype::new(&source)?;

    let destination = Utf8PathBuf::from(matches.get_one::<String>("destination").unwrap());

    archetype.check_requirements(&runtime_context)?;
    archetype.render_with_destination(destination, runtime_context, answers)?;
    Ok(())
}
