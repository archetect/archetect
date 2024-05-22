use std::io;

use clap::builder::BoolishValueParser;
use clap::{command, value_parser, Arg, ArgAction, ArgMatches, Command};
use clap_complete::{generate, Shell};
use log::Level;

use archetect_core::errors::ArchetectError;

use crate::cli;
use crate::vendor::loggerv;

pub fn command() -> Command {
    command!()
        .name("archetect")
        .help_expected(true)
        .args(render_args(false))
        .subcommand(
            Command::new("render")
                .about("Render an Archetype")
                .long_about("Render an Archetype from an archetype or catalog directory or git URL")
                .arg(
                    Arg::new("source")
                        .help("The Archetype or Catalog source directory or git URL")
                        .action(ArgAction::Set)
                        .required(true),
                )
                .arg(
                    Arg::new("destination")
                        .help("The directory to render the Archetype in to")
                        .default_value(".")
                        .action(ArgAction::Set),
                )
                .args(render_args(true)),
        )
        .subcommand(
            Command::new("catalog")
                .about("Render an Archetype from a Catalog")
                .hide(true)
                .arg(
                    Arg::new("source")
                        .help("The Catalog source directory or git URL")
                        .action(ArgAction::Set)
                        .required(true),
                )
                .arg(
                    Arg::new("destination")
                        .help("The directory to render the Archetype in to")
                        .default_value(".")
                        .action(ArgAction::Set),
                )
                .args(render_args(true)),
        )
        .subcommand(
            Command::new("config")
                .arg_required_else_help(true)
                .about("Manage Archetect's configuration")
                .subcommand(Command::new("merged").about(
                    "Show Archetect's merged configuration after applying command line arguments, \
                    environment variables, and configuration files.",
                ))
                .subcommand(Command::new("defaults").about(
                    "Show Archetect's default configuration, which may be used for re-creating \
                    a configuration file.",
                ))
                .subcommand(Command::new("edit").about("Open Archetect's config file in an editor"))
                .args(render_args(true)),
        )
        .arg(
            Arg::new("action")
                .help("Execute a configured actions")
                .long_help("Execute a configured action defined within an archetect.yaml file")
                .default_value("default")
                .action(ArgAction::Set)
                .global(true)
        )
        .subcommand(
            Command::new("actions")
                .about("List configured actions")
        )
        .subcommand(
            Command::new("server")
                .about("Start Archetect Server")
                .arg(
                    Arg::new("server-port")
                        .long("port")
                        .short('p')
                        .action(ArgAction::Set)
                        .value_parser(value_parser!(i64).range(1024..65535))
                        .env("ARCHETECT_SERVER_PORT")
                )
        )
        .arg(
            Arg::new("verbosity")
                .help("Increase verbosity level")
                .short('v')
                .long("verbose")
                .action(ArgAction::Count)
                .global(true),
        )
        .arg(
            Arg::new("config-file")
                .help("Supply an additional configuration file.")
                .long_help(
                    "Supply an additional configuration file to supplement or override \
                user and/or default configuration.",
                )
                .long("config-file")
                .short('c')
                .action(ArgAction::Set)
                .global(true)
                .value_name("config"),
        )
        .arg(
            Arg::new("destination")
                .help("The directory to render the Archetype in to")
                .long("destination")
                .visible_alias("dest")
                .default_value(".")
                .action(ArgAction::Set),
        )
        .subcommand(
            Command::new("completions")
                .about("Generate shell completions")
                .arg_required_else_help(true)
                .arg(
                    Arg::new("generator")
                        .help("Shell Generator")
                        .value_parser(value_parser!(Shell)),
                ),
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
                            Command::new("config").about("The location where archetect config files are stored."),
                        )
                        .subcommand(Command::new("answers").about("The location where answers are specified.")),
                ),
        )
        .subcommand(
            Command::new("cache")
                .about("Manage/Select from Archetypes cached from Git Repositories")
                .subcommand(Command::new("manage").about("Manage Archetect's cache for entries defined within actions")
                    .arg(
                        Arg::new("action")
                            .help("The action to managed")
                            .long_help("The action to managed, as defined within an archetype.yaml")
                            .default_value("default")
                            .action(ArgAction::Set)
                    )
                )
                .subcommand(Command::new("clear").about("Removes Archetect's entire Repository Cache"))
                .subcommand(Command::new("pull").about("Pull all Archetypes and Catalogs in Archetect's Catalog")),
        )
        .allow_external_subcommands(true)
}

fn render_args(global: bool) -> Vec<Arg> {
    let mut args = vec![];
    args.push(
        Arg::new("answer")
            .help("Supply a key=value pair as an answer to a variable question.")
            .long_help(VALID_ANSWER_INPUTS)
            .long("answer")
            .short('a')
            .action(ArgAction::Append)
            .value_name("prompt key=value")
            .global(global),
    );

    args.push(
        Arg::new("answer-file")
            .help("Supply an answers file in JSON, YAML, or Rhai format as answers to variable questions.")
            .long_help(
                "Supply an answers file in JSON, YAML, or Rhai format as answers to variable questions. This option may \
                     be specified more than once.",
            )
            .long("answer-file")
            .short('A')
            .action(ArgAction::Append)
            .value_name("path")
            .global(global)
        ,
    );

    args.push(
        Arg::new("use-defaults")
            .help("Use the configured default value for a prompt key")
            .long("use-default")
            .short('d')
            .value_delimiter(',')
            .action(ArgAction::Append)
            .value_name("prompt key")
            .global(global),
    );

    args.push(
        Arg::new("use-defaults-all")
            .help("Use the configured default values for all prompts without explicit answers")
            .long("use-defaults-all")
            .short('D')
            .alias("use-defaults-unanswered")
            .action(ArgAction::SetTrue)
            .global(global),
    );

    args.push(
        Arg::new("switches")
            .help("Enable switches that may trigger functionality within Archetypes")
            .long("switch")
            .short('s')
            .action(ArgAction::Append)
            .value_name("switch name")
            .global(global),
    );

    args.push(
        Arg::new("offline")
            .help("Only use directories and already-cached remote git URLs")
            .short('o')
            .long("offline")
            .env("ARCHETECT_OFFLINE")
            .action(ArgAction::SetTrue)
            .global(global),
    );
    args.push(
        Arg::new("allow-exec")
            .help("Allow Archetypes to execute arbitrary commands")
            .long("allow-exec")
            .alias("ae")
            .short('e')
            .env("ARCHETECT_ALLOW_EXEC")
            .action(ArgAction::Set)
            .default_missing_value("true")
            .default_value("false")
            .num_args(0..=1)
            .value_parser(BoolishValueParser::new())
            .global(global),
    );
    args.push(
            Arg::new("headless")
                .help("Expect all inputs to be resolved by answers, defaults, and optional values, never waiting on interactive user input.")
                .long("headless")
                .env("ARCHETECT_HEADLESS")
                .action(ArgAction::SetTrue)
                .global(global)
        );
    args.push(
        Arg::new("local")
            .help("Use local development checkouts where available and configured")
            .long("local")
            .short('l')
            .env("ARCHETECT_LOCAL")
            .action(ArgAction::SetTrue)
            .global(global),
    );
    args.push(
        Arg::new("force-update")
            .help("Force updates for all Catalogs and Archetypes when rendering")
            .long("force-update")
            .short('U')
            .env("ARCHETECT_FORCE_UPDATE")
            .action(ArgAction::SetTrue)
            .global(global),
    );
    args
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
