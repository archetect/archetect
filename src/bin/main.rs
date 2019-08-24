#[macro_use]
extern crate clap;

use archetect::config::{
    Answer, AnswerConfig, AnswerConfigError, ArchetypeConfig, CatalogConfig, CatalogConfigError, Variable,
};
use archetect::util::paths;
use archetect::util::{Source, SourceError};
use archetect::{self, Archetype};
use clap::{App, AppSettings, Arg, SubCommand};
use indoc::indoc;
use log::{error, info};
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::str::FromStr;

fn main() {
    let matches = App::new(&crate_name!()[..])
        .version(&crate_version!()[..])
        .author("Jimmie Fulton <jimmie.fulton@gmail.com")
        .about("Generates Projects and Files from Archetype Template Directories and Git Repositories.")
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
                    SubCommand::with_name("paths")
                        .about("Get paths ")
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
                .subcommand(SubCommand::with_name("pull"))
                .subcommand(SubCommand::with_name("path")),
        )
        .subcommand(
            SubCommand::with_name("create")
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
                .arg(
                    Arg::with_name("offline")
                        .help("Only use directories and already-cached remote git URLs")
                        .short("o")
                        .long("offline"),
                ),
        )
        .get_matches();

    archetect::loggerv::Logger::new()
        .verbosity(matches.occurrences_of("verbosity"))
        .level(false)
        .prefix("archetect")
        .no_module_path()
        .module_path(false)
        .base_level(log::Level::Info)
        .init()
        .unwrap();

    let mut answers = HashMap::new();

    if let Ok(user_answers) = AnswerConfig::load(paths::answers_config()) {
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
        let git_cache = paths::git_cache_dir();
        if let Some(_sub_matches) = matches.subcommand_matches("clear") {
            fs::remove_dir_all(&git_cache).expect("Error deleting archetect cache");
        }
        if let Some(_) = matches.subcommand_matches("path") {
            println!("{}", git_cache.display());
        }
    }

    if let Some(matches) = matches.subcommand_matches("system") {
        if let Some(matches) = matches.subcommand_matches("paths") {
            if let Some(_) = matches.subcommand_matches("git") {
                println!("{}", paths::git_cache_dir().display());
            }
            if let Some(_) = matches.subcommand_matches("catalogs") {
                println!("{}", paths::catalog_cache_dir().display());
            }
            if let Some(_) = matches.subcommand_matches("config") {
                println!("{}", paths::configs_dir().display());
            }
        }
    }

    if let Some(matches) = matches.subcommand_matches("create") {
        let source = matches.value_of("source").unwrap();
        let destination = PathBuf::from_str(matches.value_of("destination").unwrap()).unwrap();
        let offline: bool = matches.is_present("offline");

        match Source::detect(source, offline, None) {
            Ok(source) => {
                let archetype = Archetype::from_source(source, offline).unwrap();
                if let Ok(answer_config) = AnswerConfig::load(destination.clone()) {
                    for answer in answer_config.answers() {
                        if !answers.contains_key(answer.identifier()) {
                            let answer = answer.clone();
                            answers.insert(answer.identifier().to_owned(), answer);
                        }
                    }
                }
                let context = archetype.get_context(&answers).unwrap();
                archetype.render(destination, context).unwrap();
            }
            Err(err) => match err {
                SourceError::SourceInvalidEncoding => error!("\"{}\" is not valid UTF-8", source),
                SourceError::SourceNotFound => error!("\"{}\" does not exist", source),
                SourceError::SourceUnsupported => error!("\"{}\" is not a supported archetype path", source),
                SourceError::SourceInvalidPath => error!("\"{}\" is not a valid archetype path", source),
                SourceError::OfflineAndNotCached => error!(
                    "\"{}\" is not cached locally and cannot be cloned in offline mode",
                    source
                ),
                SourceError::RemoteSourceError(err) => error!("Remote Source Error\n{}", err),
                SourceError::IOError(err) => error!("IO Error: {}", err),
            },
        }
    } else if let Some(matches) = matches.subcommand_matches("archetype") {
        if let Some(matches) = matches.subcommand_matches("init") {
            let output_dir = PathBuf::from_str(matches.value_of("destination").unwrap()).unwrap();
            if !output_dir.exists() {
                fs::create_dir_all(&output_dir).unwrap();
            }

            let mut config = ArchetypeConfig::default();
            config.add_variable(Variable::with_identifier("name").with_prompt("Application Name: "));
            config.add_variable(Variable::with_identifier("author").with_prompt("Author name: "));

            let mut config_file = File::create(output_dir.clone().join("archetype.toml")).unwrap();
            config_file
                .write(toml::ser::to_string_pretty(&config).unwrap().as_bytes())
                .unwrap();

            File::create(output_dir.clone().join("README.md")).expect("Error creating archetype README.md");
            File::create(output_dir.clone().join(".gitignore")).expect("Error creating archetype .gitignore");

            let project_dir = output_dir.clone().join("archetype/{{ name # train_case }}");

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
}
