use std::collections::HashSet;
use std::ops::Deref;

use camino::Utf8PathBuf;
use clap::ArgMatches;
use log::error;
use read_input::prelude::*;
use rhai::{Dynamic, EvalAltResult, Map};

use archetect_core::{self};
use archetect_core::Archetect;
use archetect_core::configuration::Configuration;
use archetect_core::errors::ArchetectError;
use archetect_core::source::Source;
use archetect_core::v2::archetype::archetype::Archetype;
use archetect_core::v2::catalog::Catalog;
use archetect_core::v2::runtime::context::RuntimeContext;

use crate::answers::parse_answer_pair;

mod answers;
mod cli;
pub mod configuration;
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

    for (identifier, value) in configuration.answers() {
        answers.insert(identifier.clone(), value.clone());
    }

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
            let (identifier, value) = parse_answer_pair(answer_match).unwrap();
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
    let runtime_context = RuntimeContext::new(configuration, switches, destination);

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
    let source = Source::detect(&archetect, &runtime_context, source, None)?;

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
    let source = Source::detect(&archetect, &runtime_context, source, None)?;

    let archetype = Archetype::new(&source)?;

    let destination = Utf8PathBuf::from(matches.get_one::<String>("destination").unwrap());

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
