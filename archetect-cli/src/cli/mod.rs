use archetect_core::config::{AnswerConfig, AnswerConfigError, AnswerInfo};
use archetect_core::loggerv;
use clap::{crate_authors, crate_description, crate_version};
use clap::{App, AppSettings, Arg, ArgMatches, SubCommand};
use archetect_core::loggerv::Output;
use log::Level;

pub fn get_matches() -> App<'static, 'static> {
    App::new("archetect")
        .version(crate_version!())
        .author(crate_authors!())
        .about(crate_description!())
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .arg(
            Arg::with_name("verbosity")
                .short("v")
                .long("verbose")
                .multiple(true)
                .global(true)
                .help("Increases the level of verbosity"),
        )
        .arg(
            Arg::with_name("offline")
                .global(true)
                .help("Only use directories and already-cached remote git URLs")
                .short("o")
                .long("offline"),
        )
        .arg(
            Arg::with_name("answer")
                .short("a")
                .long("answer")
                .takes_value(true)
                .multiple(true)
                .global(true)
                .empty_values(false)
                .value_name("key=value")
                .help("Supply a key=value pair as an answer to a variable question.")
                .long_help(VALID_ANSWER_INPUTS)
                .validator(|s| match AnswerInfo::parse(&s) {
                    Ok(_) => Ok(()),
                    _ => Err(format!(
                        "'{}' is not in a proper key=value answer format. \n{}",
                        s, VALID_ANSWER_INPUTS
                    )),
                }),
        )
        .arg(
            Arg::with_name("switches")
                .short("s")
                .long("switch")
                .takes_value(true)
                .multiple(true)
                .global(true)
                .empty_values(true)
                .help("Enable switches that may trigger functionality within Archetypes")
        )
        .arg(
            Arg::with_name("answer-file")
                .short("A")
                .long("answer-file")
                .takes_value(true)
                .multiple(true)
                .global(true)
                .empty_values(false)
                .value_name("path")
                .help("Supply an answers file as answers to variable questions.")
                .long_help(
                    "Supply an answers file as answers to variable questions. This option may \
                     be specified more than once.",
                )
                .validator(|af| match AnswerConfig::load(&af) {
                    Ok(_) => Ok(()),
                    Err(AnswerConfigError::ParseError(_)) => Err(format!("{} has an invalid answer file format", &af)),
                    Err(AnswerConfigError::MissingError) => {
                        Err(format!("{} does not exist or does not contain an answer file", &af))
                    }
                }),
        )
        .subcommand(
            SubCommand::with_name("catalog")
                .about("Select From a Catalog")
                .arg(
                    Arg::with_name("destination")
                        .default_value(".")
                        .help("The directory to render the Archetype in.")
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("source")
                        .long("source")
                        .short("S")
                        .takes_value(true)
                        .help("Catalog source location")
                    ,
                )
            ,
        )
        .subcommand(
            SubCommand::with_name("completions")
                .about("Generate shell completions")
                .setting(AppSettings::SubcommandRequiredElseHelp)
                .subcommand(SubCommand::with_name("fish").about("Generate Fish Shell completions"))
                .subcommand(SubCommand::with_name("zsh").about("Generate ZSH completions"))
                .subcommand(SubCommand::with_name("bash").about("Generate Bash Shell completions"))
                .subcommand(SubCommand::with_name("powershell").about("Generate PowerShell completions")),
        )
        .subcommand(
            SubCommand::with_name("system")
                .about("archetect system configuration")
                .subcommand(
                    SubCommand::with_name("layout")
                        .about("Get layout of system paths")
                        .subcommand(
                            SubCommand::with_name("git")
                                .about("The location where git repos are cloned.  Used for offline mode."),
                        )
                        .subcommand(
                            SubCommand::with_name("http")
                                .about("The location where http resources are cached.  Used for offline mode."),
                        )
                        .subcommand(
                            SubCommand::with_name("config")
                                .about("The location where archetect config files are stored."),
                        )
                        .subcommand(
                            SubCommand::with_name("answers").about("The location where answers are specified."),
                        ),
                ),
        )
        .subcommand(
            SubCommand::with_name("cache")
                .about("Manage/Select from Archetypes cached from Git Repositories")
                .subcommand(SubCommand::with_name("select"))
                .subcommand(SubCommand::with_name("clear"))
                .subcommand(SubCommand::with_name("pull")),
        )
        .subcommand(
            SubCommand::with_name("render")
                .alias("create")
                .about("Creates content from an Archetype")
                .arg(
                    Arg::with_name("source")
                        .help("The Archetype source directory or git URL")
                        .takes_value(true)
                        .required(true),
                )
                .arg(
                    Arg::with_name("destination")
                        .default_value(".")
                        .help("The directory the Archetype should be rendered into.")
                        .takes_value(true),
                ),
        )
}

pub fn configure(matches: &ArgMatches) {
    loggerv::Logger::new()
        .output(&Level::Error, Output::Stderr)
        .output(&Level::Warn, Output::Stderr)
        .output(&Level::Info, Output::Stderr)
        .output(&Level::Debug, Output::Stderr)
        .output(&Level::Trace, Output::Stderr)
        .verbosity(matches.occurrences_of("verbosity"))
        .level(false)
        .prefix("archetect")
        .no_module_path()
        .module_path(false)
        .base_level(log::Level::Info)
        .init()
        .unwrap();
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
