use std::io;

use archetect_core::ArchetectError;
use clap::{command, value_parser, Arg, ArgAction, ArgMatches, Command};
use clap_complete::{generate, Shell};
use log::Level;

use crate::cli;

use crate::vendor::loggerv;

pub fn command() -> Command {
    command!()
        .name("archetect")
        .help_expected(true)
        .arg(
            Arg::new("verbosity")
                .help("Increase verbosity level")
                .short('v')
                .long("verbose")
                .action(ArgAction::Count)
                .global(true)
        )
        .arg(
            Arg::new("offline")
                .help("Only use directories and already-cached remote git URLs")
                .short('o')
                .long("offline")
                .action(ArgAction::SetTrue)
                .global(true)
        )
        .arg(
            Arg::new("headless")
                .help("Expect all variable values to be provided by answer arguments or files, never waiting for user input.")
                .long("headless")
                .action(ArgAction::SetTrue)
                .global(true)
        )
        .arg(
            Arg::new("local")
                .help("Use local development checkouts where available and configured")
                .long("local")
                .action(ArgAction::SetTrue)
                .global(true)
        )
        .arg(
            Arg::new("answer")
                .help("Supply a key=value pair as an answer to a variable question.")
                .long_help(VALID_ANSWER_INPUTS)
                .long("answer")
                .short('a')
                .action(ArgAction::Append)
                .value_name("key=value")
                .global(true)
        )
        .arg(
            Arg::new("switches")
                .help("Enable switches that may trigger functionality within Archetypes")
                .long("switch")
                .short('s')
                .action(ArgAction::Append)
                .global(true)
        )
        .arg(
            Arg::new("answer-file")
                .help("Supply an answers file as answers to variable questions.")
                .long_help(
                    "Supply an answers file as answers to variable questions. This option may \
                     be specified more than once.",
                )
                .long("answer-file")
                .short('A')
                .action(ArgAction::Append)
                .global(true)
                .value_name("path")
                // .value_parser(ValueParser::new(parse_answer_file))
        )
        .arg(
            Arg::new("destination")
                .help("The directory to render the Archetype in")
                .default_value(".")
                .action(ArgAction::Set)
        )
        .subcommand(
            Command::new("catalog")
                .about("Select From a Catalog")
                .arg(
                    Arg::new("destination")
                        .help("The directory to render the Archetype in")
                        .default_value(".")
                        .action(ArgAction::Set)
                )
                .arg(
                    Arg::new("source")
                        .help("Catalog source location")
                        .long("source")
                        .short('S')
                        .action(ArgAction::Set)
                ),
        )
        .subcommand(
            Command::new("completions")
                .about("Generate shell completions")
                .arg_required_else_help(true)
                .arg(
                    Arg::new("generator")
                        .help("Shell Generator")
                        .value_parser(value_parser!(Shell))
                )
        )
        .subcommand(
            Command::new("system")
                .about("archetect system configuration")
                .subcommand(
                    Command::new("layout")
                        .about("Get layout of system paths")
                        .subcommand(
                            Command::new("git")
                                .about("The location where git repos are cloned.  Used for offline mode."),
                        )
                        .subcommand(
                            Command::new("http")
                                .about("The location where http resources are cached.  Used for offline mode."),
                        )
                        .subcommand(
                            Command::new("config")
                                .about("The location where archetect config files are stored."),
                        )
                        .subcommand(
                            Command::new("answers").about("The location where answers are specified."),
                        ),
                ),
        )
        .subcommand(
            Command::new("cache")
                .about("Manage/Select from Archetypes cached from Git Repositories")
                .subcommand(Command::new("select"))
                .subcommand(Command::new("clear"))
                .subcommand(Command::new("pull")),
        )
        .subcommand(
            Command::new("render")
                .alias("create")
                .about("Creates content from an Archetype")
                .arg(
                    Arg::new("source")
                        .help("The Archetype source directory or git URL")
                        .action(ArgAction::Set)
                        .required(true),
                )
                .arg(
                    Arg::new("destination")
                        .help("The directory the Archetype should be rendered into.")
                        .default_value(".")
                        .action(ArgAction::Set)
                ),
        )
}

pub fn configure(matches: &ArgMatches) {
    loggerv::Logger::new()
        .output(&Level::Error, loggerv::Output::Stderr)
        .output(&Level::Warn, loggerv::Output::Stderr)
        .output(&Level::Info, loggerv::Output::Stderr)
        .output(&Level::Debug, loggerv::Output::Stderr)
        .output(&Level::Trace, loggerv::Output::Stderr)
        .verbosity(matches.get_count("verbosity") as u64)
        .level(false)
        .prefix("archetect")
        .no_module_path()
        .module_path(false)
        .base_level(Level::Info)
        .init()
        .unwrap();
}

pub fn completions(matches: &ArgMatches) -> Result<(), ArchetectError> {
    if let Some(generator) = matches.get_one::<Shell>("generator") {
        let mut command = cli::command();
        eprintln!("Generating completions for {generator}");
        generate(*generator, &mut command, "archetect", &mut io::stdout());
    } else {
        return Err(ArchetectError::GeneralError("Invalid completions shell".to_owned()));
    }

    Ok(())
}

const VALID_ANSWER_INPUTS: &str = "Supply a key=value pair as an answer to a variable question. \
                                   This option may be specified more than once.\n\
                                   \nValid Input Examples:\n\
                                   \nkey=value\
                                   \nkey='multi-word value'\
                                   \nkey=\"multi-word value\"\
                                   \n\"key=value\"\
                                   \n'key=value'\
                                   \n'key=\"multi-word value\"'\
                                   \n\"key = 'multi-word value'\"\
                                   ";
