use archetect::config::{
    Answer, AnswerConfig, AnswerConfigError, ArchetypeConfig, CatalogConfig, CatalogConfigError, Variable,
};
use archetect::system::SystemError;
use archetect::util::SourceError;
use archetect::{self, ArchetypeError, ArchetectError};
use clap::{App, AppSettings, Arg, SubCommand, ArgMatches};
use clap::{crate_name, crate_description, crate_authors, crate_version};
use indoc::indoc;
use log::{error, info};
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::str::FromStr;
use archetect::RenderError;
use std::error::Error;

pub mod loggerv;

fn main() {
    let matches = App::new(crate_name!())
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
                .long_help(
                    format!(
                        "Supply a key=value pair as an answer to a variable question. \
                         This option may be specified more than once.\n{}",
                        VALID_ANSWER_INPUTS
                    )
                        .as_str(),
                )
                .validator(|s| match Answer::parse(&s) {
                    Ok(_) => Ok(()),
                    _ => Err(format!(
                        "'{}' is not in a proper key=value answer format. \n{}",
                        s, VALID_ANSWER_INPUTS
                    )),
                }),
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
            SubCommand::with_name("archetype")
                .about("Archetype Authoring Tools")
                .setting(AppSettings::SubcommandRequiredElseHelp)
                .subcommand(
                    SubCommand::with_name("init").about("Creates a minimal template").arg(
                        Arg::with_name("destination")
                            .takes_value(true)
                            .help("Destination")
                            .required(true),
                    ),
                ),
        )
        .subcommand(
            SubCommand::with_name("catalog")
                .about("Create/Manage/Select From a Catalog of Archetypes")
                .subcommand(
                    SubCommand::with_name("add")
                        .arg(
                            Arg::with_name("source")
                                .short("l")
                                .long("source")
                                .takes_value(true)
                                .help("Archetype source location"),
                        )
                        .arg(
                            Arg::with_name("description")
                                .short("d")
                                .long("description")
                                .takes_value(true)
                                .help("Archetype Description"),
                        ),
                )
                .subcommand(
                    SubCommand::with_name("select").arg(
                        Arg::with_name("catalog-file")
                            .short("c")
                            .long("catalog-file")
                            .takes_value(true)
                            .required(true),
                    ),
                ),
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
                            SubCommand::with_name("config")
                                .about("The location where archetect config files are stored."),
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
                        .help("The source archetype directory or git URL")
                        .takes_value(true)
                        .required(true),
                )
                .arg(
                    Arg::with_name("destination")
                        .default_value(".")
                        .help("The directory to initialize the Archetype template in.")
                        .takes_value(true),
                )
        )
        .get_matches();

    loggerv::Logger::new()
        .verbosity(matches.occurrences_of("verbosity"))
        .level(false)
        .prefix("archetect")
        .no_module_path()
        .module_path(false)
        .base_level(log::Level::Info)
        .init()
        .unwrap();

    match execute(matches) {
        Ok(()) => (),
        Err(error) => handle_archetect_error(error),
    }
}

fn execute(matches: ArgMatches) -> Result<(), ArchetectError> {
    let archetect = archetect::Archetect::builder()
        .with_offline(matches.is_present("offline"))
        .build()?;

    let mut answers = HashMap::new();

    if let Ok(user_answers) = AnswerConfig::load(archetect.layout().answers_config()) {
        for answer in user_answers.answers() {
            let answer = answer.clone();
            answers.insert(answer.identifier().to_owned(), answer);
        }
    }

    if let Some(matches) = matches.values_of("answer-file") {
        for f in matches.map(|m| AnswerConfig::load(m).unwrap()) {
            for answer in f.answers() {
                let answer = answer.clone();
                answers.insert(answer.identifier().to_string(), answer);
            }
        }
    }

    if let Some(matches) = matches.values_of("answer") {
        for a in matches.map(|m| Answer::parse(m).unwrap()) {
            answers.insert(a.identifier().to_string(), a);
        }
    }

    if let Some(matches) = matches.subcommand_matches("cache") {
        let git_cache = archetect.layout().git_cache_dir();
        if let Some(_sub_matches) = matches.subcommand_matches("clear") {
            fs::remove_dir_all(&git_cache).expect("Error deleting archetect cache");
        }
    }

    if let Some(matches) = matches.subcommand_matches("system") {
        if let Some(matches) = matches.subcommand_matches("layout") {
            match matches.subcommand() {
                ("git", Some(_)) => println!("{}", archetect.layout().git_cache_dir().display()),
                ("catalogs", Some(_)) => println!("{}", archetect.layout().catalog_cache_dir().display()),
                ("config", Some(_)) => println!("{}", archetect.layout().configs_dir().display()),
                _ => println!("{}", archetect.layout()),
            }
        }
    }

    if let Some(matches) = matches.subcommand_matches("render") {
        let source = matches.value_of("source").unwrap();
        let destination = PathBuf::from_str(matches.value_of("destination").unwrap()).unwrap();

        let archetype = archetect.load_archetype(source, None)?;

        if let Ok(answer_config) = AnswerConfig::load(destination.clone()) {
            for answer in answer_config.answers() {
                if !answers.contains_key(answer.identifier()) {
                    let answer = answer.clone();
                    answers.insert(answer.identifier().to_owned(), answer);
                }
            }
        }
        let context = archetype.get_context(&answers, None).unwrap();
        return archetype.render(destination, context).map_err(|e| e.into());
    } else if let Some(matches) = matches.subcommand_matches("archetype") {
        if let Some(matches) = matches.subcommand_matches("init") {
            let output_dir = PathBuf::from_str(matches.value_of("destination").unwrap()).unwrap();
            if !output_dir.exists() {
                fs::create_dir_all(&output_dir).unwrap();
            }

            let mut config = ArchetypeConfig::default();
            config.add_variable(Variable::with_name("name").with_prompt("Application Name: "));
            config.add_variable(Variable::with_name("author").with_prompt("Author name: "));

            let mut config_file = File::create(output_dir.clone().join("archetype.toml")).unwrap();
            config_file
                .write(toml::ser::to_string_pretty(&config).unwrap().as_bytes())
                .unwrap();

            File::create(output_dir.clone().join("README.md")).expect("Error creating archetype README.md");
            File::create(output_dir.clone().join(".gitignore")).expect("Error creating archetype .gitignore");

            let project_dir = output_dir.clone().join("contents/{{ name # train_case }}");

            fs::create_dir_all(&project_dir).unwrap();

            let mut project_readme =
                File::create(project_dir.clone().join("README.md")).expect("Error creating project README.md");
            project_readme
                .write_all(
                    indoc!(
                        r#"
                        Project: {{ name | title_case }}
                        Author: {{ author | title_case }}
                    "#
                    )
                        .as_bytes(),
                )
                .expect("Error writing README.md");
            File::create(project_dir.clone().join(".gitignore")).expect("Error creating project .gitignore");
        }
    }

    if let Some(matches) = matches.subcommand_matches("catalog") {
        if let Some(sub_matches) = matches.subcommand_matches("select") {
            let catalog_path = sub_matches.value_of("catalog-file").unwrap();
            match CatalogConfig::load(catalog_path) {
                Ok(catalog) => {
                    info!("Catalog loaded successfully!");
                    if let Ok(archetype_info) = archetect::input::select_from_catalog(&catalog) {
                        println!("{} selected", archetype_info.description());
                    }
                }
                Err(CatalogConfigError::CatalogConfigTomlParseError(cause)) => error!(
                    "Error parsing catalog '{}'. \
                     Cause: {}",
                    catalog_path, cause
                ),
                Err(CatalogConfigError::IOError(cause)) => {
                    error!("Error reading catalog '{}'. Cause: {}", catalog_path, cause)
                }
            }
        }
    }

    Ok(())
}

fn handle_archetect_error(error: ArchetectError) {
    match error {
        ArchetectError::SourceError(error) => handle_source_error(error),
        ArchetectError::ArchetypeError(error) => handle_archetype_error(error),
        ArchetectError::GenericError(error) => error!("Archetect Error: {}", error),
        ArchetectError::RenderError(error) => handle_render_error(error),
        ArchetectError::SystemError(error) => handle_system_error(error),
    }
}

fn handle_archetype_error(error: ArchetypeError) {
    match error {
        ArchetypeError::ArchetypeInvalid => panic!(),
        ArchetypeError::InvalidAnswersConfig => panic!(),
        ArchetypeError::RenderError(error) => handle_render_error(error),
        ArchetypeError::ArchetypeSaveFailed => {}
        ArchetypeError::SourceError(error) => handle_source_error(error),
    }
}

fn handle_source_error(error: SourceError) {
    match error {
        SourceError::SourceInvalidEncoding(source) => error!("\"{}\" is not valid UTF-8", source),
        SourceError::SourceNotFound(source) => error!("\"{}\" does not exist", source),
        SourceError::SourceUnsupported(source) => error!("\"{}\" is not a supported archetype path", source),
        SourceError::SourceInvalidPath(source) => error!("\"{}\" is not a valid archetype path", source),
        SourceError::OfflineAndNotCached(source) => error!(
            "\"{}\" is not cached locally and cannot be cloned in offline mode",
            source
        ),
        SourceError::RemoteSourceError(err) => error!("Remote Source Error\n{}", err),
        SourceError::IOError(err) => error!("IO Error: {}", err),
    };
}

fn handle_system_error(error: SystemError) {
    match error {
        SystemError::GenericError(error) => error!("System Error: {}", error),
        SystemError::IOError { error, message: _ } => error!("{}", error.to_string()),
    }
}

fn handle_render_error(error: RenderError) {
    match error {
        RenderError::FileRenderError { source, error, message: _ } => {
            if let Some(cause) = error.source() {
                error!("{} in template \"{}\"", cause, source.display());
            } else {
                error!("Error rendering template \"{}\"\n\n{}", source.display(), error);
            }
        }
        RenderError::FileRenderIOError { source, error, message: _ } => {
            error!("IO Error: {} in template \"{}\"", error, source.display());
        }
        RenderError::PathRenderError { source, error, message: _ } => {
            if let Some(cause) = error.source() {
                error!("{} in path \"{}\"", cause, source.display());
            } else {
                error!("Error rendering path name \"{}\"\n\n{:?}", source.display(), error);
            }
        }
        RenderError::StringRenderError { source, error: _, message } => {
            error!("IO Error: {} in \"{}\"", message, source);
        }
        RenderError::IOError { error: _, message } => {
            error!("Unexpected IO Error:\n{}", message);
        }
    }
}

const VALID_ANSWER_INPUTS: &str = "\
                                       \nValid Input Examples:\n\
                                       \nkey=value\
                                       \nkey='multi-word value'\
                                       \nkey=\"multi-word value\"\
                                       \n\"key=value\"\
                                       \n'key=value'\
                                       \n'key=\"multi-word value\"''\
                                       \n\"key = 'multi-word value'\"\
                                       ";
