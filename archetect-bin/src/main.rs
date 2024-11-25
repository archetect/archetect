use std::collections::HashSet;

use camino::Utf8PathBuf;
use clap::ArgMatches;
use log::warn;
use rhai::Map;

use archetect_api::{CommandRequest, IoDriver};
use archetect_core::{self};
use archetect_core::actions::ArchetectAction;
use archetect_core::Archetect;
use archetect_core::archetype::archetype::Archetype;
use archetect_core::archetype::render_context::RenderContext;
use archetect_core::catalog::{Catalog, CatalogManifest};
use archetect_core::configuration::Configuration;
use archetect_core::errors::{ArchetectError, ArchetypeError, CatalogError, SourceError};
use archetect_core::source::SourceContents;
use archetect_core::system::{RootedSystemLayout, SystemLayout};
use archetect_terminal_io::TerminalIoDriver;
use ArchetypeError::ScriptAbortError;

use crate::answers::parse_answer_pair;
use crate::subcommands::handle_commands_subcommand;

mod answers;
mod cli;
mod configuration;
mod subcommands;
pub mod vendor;

fn main() {
    let matches = cli::command()
        .get_matches();
    cli::configure(&matches);

    let driver = TerminalIoDriver::default();
    let layout = RootedSystemLayout::dot_home().unwrap();

    match execute(matches, driver.clone(), layout) {
        Ok(()) => (),
        Err(error) => {
            match error {
                // Handled when the Rhai script ends by the IO Driver
                ArchetectError::ArchetypeError(ScriptAbortError) => {}
                ArchetectError::CatalogError(CatalogError::SelectionCancelled) => {}
                _ => {
                    driver.send(CommandRequest::LogError(format!("{}", error)));
                }
            }

            std::process::exit(-1);
        }
    }
}

fn execute<D: IoDriver, L: SystemLayout>(matches: ArgMatches, driver: D, layout: L) -> Result<(), ArchetectError> {
    let configuration = configuration::load_user_config(&layout, &matches)
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

    let archetect = Archetect::builder()
        .with_configuration(configuration)
        .with_driver(driver)
        .with_layout(layout)
        .build()?;

    match matches.subcommand() {
        Some(("completions", args)) => cli::completions(args)?,
        Some(("actions", args)) => handle_commands_subcommand(args, &archetect),
        Some(("render", args)) => render(args, archetect, answers)?,
        Some(("catalog", args)) => catalog(args, archetect, answers)?,
        Some(("config", args)) => subcommands::handle_config_subcommand(args, &archetect)?,
        Some(("cache", args)) => subcommands::handle_cache_subcommand(args, &archetect)?,
        Some(("check", args)) => subcommands::handle_check_subcommand(args, &archetect)?,
        Some((_, _args)) => {
            execute_action(&matches, archetect, answers)?;
        },
        None => {
            execute_action(&matches, archetect, answers)?;
        }
    }

    Ok(())
}

fn execute_action(matches: &ArgMatches, archetect: Archetect, answers: Map) -> Result<(), ArchetectError> {
    let action = matches.get_one::<String>("action").expect("Expected an action");
    match archetect.configuration().action(&action) {
        None => {
            Err(ArchetectError::MissingAction(
                action.to_owned(),
                archetect
                    .configuration()
                    .actions()
                    .keys()
                    .map(|v| v.to_string())
                    .collect::<Vec<String>>(),
            ))
        }
        Some(command) => {
            match command {
                ArchetectAction::RenderGroup{info, ..} => {
                    let catalog = Catalog::new(archetect.clone(), CatalogManifest::new().with_entries(info.actions().clone()));
                    let destination = shellexpand::full(matches.get_one::<String>("destination").expect("Enforced by Clap"))?.to_string();
                    let destination = Utf8PathBuf::from(destination);
                    let render_context = configure_render_context(RenderContext::new(destination, answers), &archetect, matches);
                    catalog.render(render_context)?;
                }
                ArchetectAction::RenderCatalog{info, ..} => {
                    let destination = shellexpand::full(matches.get_one::<String>("destination").expect("Enforced by Clap"))?.to_string();
                    let destination = Utf8PathBuf::from(destination);
                    let render_context = configure_render_context(RenderContext::new(destination, answers), &archetect, matches);
                    let catalog = archetect.new_catalog(info.source())?;
                    catalog.check_requirements()?;
                    catalog.render(render_context)?;
                }
                ArchetectAction::RenderArchetype{info, ..} => {
                    let destination = shellexpand::full(matches.get_one::<String>("destination").expect("Enforced by Clap"))?.to_string();
                    let destination = Utf8PathBuf::from(destination);
                    let render_context = configure_render_context(RenderContext::new(destination, answers), &archetect, matches)
                        .with_archetype_info(&info)
                        ;
                    let archetype = archetect.new_archetype(info.source())?;
                    archetype.check_requirements()?;
                    let _ = archetype.render(render_context)?;

                }
            }
            Ok(())
        }
    }
}

fn catalog(matches: &ArgMatches, archetect: Archetect, answers: Map) -> Result<(), ArchetectError> {
    warn!("'archetect catalog' is deprecated.  Use 'archetect render', instead");
    render(matches, archetect, answers)
}

pub fn render(matches: &ArgMatches, archetect: Archetect, answers: Map) -> Result<(), ArchetectError> {
    let source = matches.get_one::<String>("source").unwrap();
    let source = archetect.new_source(source)?;
    let destination = shellexpand::full(matches.get_one::<String>("destination").expect("Enforced by Clap"))?.to_string();
    let destination = Utf8PathBuf::from(destination);
    let render_context = configure_render_context(RenderContext::new(destination, answers), &archetect, matches);
    match source.source_contents() {
        SourceContents::Archetype => {
            let archetype = Archetype::new(archetect, source)?;
            archetype.check_requirements()?;
            Ok(archetype.render(render_context).map(|_| ())?)
        }
        SourceContents::Catalog => {
           let catalog = Catalog::load(archetect, source)?;
            catalog.check_requirements()?;
            Ok(catalog.render(render_context)?)
        }
        SourceContents::Unknown => {
            Err(SourceError::UnknownSourceContent.into())
        }

    }
}

fn configure_render_context(
    render_context: RenderContext,
    archetect: &Archetect,
    matches: &ArgMatches,
) -> RenderContext {
    render_context
        .with_switches(get_switches(matches, archetect.configuration()))
        .with_use_defaults_all(matches.get_flag("use-defaults-all"))
        .with_use_defaults(get_defaults(matches))
}

fn get_switches(matches: &ArgMatches, configuration: &Configuration) -> HashSet<String> {
    let mut switches = HashSet::new();
    for switch in configuration.switches() {
        switches.insert(switch.to_string());
    }
    if let Some(cli_switches) = matches.get_many::<String>("switches") {
        for switch in cli_switches {
            switches.insert(switch.to_string());
        }
    }
    switches
}

fn get_defaults(matches: &ArgMatches) -> HashSet<String> {
    let mut defaults = HashSet::new();
    if let Some(cli_defaults) = matches.get_many::<String>("use-defaults") {
        for default in cli_defaults {
            defaults.insert(default.to_string());
        }
    }
    defaults
}