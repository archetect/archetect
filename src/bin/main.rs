#[macro_use]
extern crate clap;

use archetect::{self, Archetype, ArchetypeConfig, DirectoryArchetype};
use archetect::config::{AnswerConfig, Answer, AnswerConfigError};
use clap::{App, AppSettings, Arg, SubCommand};
use indoc::indoc;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::str::FromStr;
use std::collections::HashMap;

fn main() {
    let matches = App::new(&crate_name!()[..])
        .version(&crate_version!()[..])
        .author("Jimmie Fulton <jimmie.fulton@gmail.com")
        .about("Generates Projects and Files from Archetype Template Directories and Git Repositories")
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .arg(Arg::with_name("v")
            .short("v")
            .multiple(true)
            .help("Sets the level of verbosity"))
        .arg(Arg::with_name("answer")
            .short("a")
            .long("answer")
            .takes_value(true)
            .multiple(true)
            .global(true)
            .empty_values(false)
            .value_name("key=value")
            .help("Supply a key=value pair as an answer to a variable question.")
            .long_help(format!("Supply a key=value pair as an answer to a variable question.\
                This option may be specified more than once.\n{}", VALID_ANSWER_INPUTS).as_str())
            .validator(|s| {
                match Answer::parse(&s) {
                    Ok(_) => Ok(()),
                    _ => Err(format!(
                        "'{}' is not a valid answer. \n{}", s, VALID_ANSWER_INPUTS)
                    )
                }
            })
        )
        .arg(Arg::with_name("answer_file")
            .short("f")
            .long("answer-file")
            .takes_value(true)
            .multiple(true)
            .global(true)
            .empty_values(false)
            .value_name("path")
            .help("Supply an answers file to variable questions.")
            .long_help("Supply an answers file as answers to variable questions. This option may \
                be specified more than once.")
            .validator(|af| {
                match AnswerConfig::load(&af) {
                    Ok(_) => Ok(()),
                    Err(AnswerConfigError::ParseError(_)) => Err(format!("{} has an invalid answer file format", &af)),
                    Err(AnswerConfigError::MissingError) => Err(format!("{} does not exist or does not contain an answer file", &af)),
                }
            })
        )
        .subcommand(
            SubCommand::with_name("archetype")
                .about("Archetype Authoring Tools")
                .setting(AppSettings::SubcommandRequiredElseHelp)
                .subcommand(
                    SubCommand::with_name("init")
                        .about("Creates a minimal template")
                        .arg(
                            Arg::with_name("destination")
                                .takes_value(true)
                                .help("Destination")
                                .required(true),
                        ),
                )
        )
        .subcommand(
            SubCommand::with_name("create")
                .about("Creates content from an Archetype")
                .arg(Arg::with_name("from").takes_value(true).required(true))
                .arg(
                    Arg::with_name("destination")
                        .default_value(".")
                        .help("The directory to initialize the Archetype template in.")
                        .takes_value(true),
                ),
        )
        .get_matches();

    loggerv::init_with_verbosity(matches.occurrences_of("v")).unwrap();

    let mut answers = HashMap::new();

    if let Some(matches) = matches.values_of("answer_file") {
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

    if let Some(matches) = matches.subcommand_matches("create") {
        let from = PathBuf::from_str(matches.value_of("from").unwrap()).unwrap();
        let destination = PathBuf::from_str(matches.value_of("destination").unwrap()).unwrap();
        let archetype = DirectoryArchetype::new(from).unwrap();

        if let Ok(answer_config) = AnswerConfig::load(destination.clone()) {
            for answer in answer_config.answers() {
                if !answers.contains_key(answer.identifier()) {
                    let answer = answer.clone();
                    answers.insert(answer.identifier().to_owned(), answer);
                }
            }
        }

        let context = archetype.get_context(&answers).unwrap();
        archetype.generate(destination, context).unwrap();
    } else if let Some(matches) = matches.subcommand_matches("archetype") {
        if let Some(matches) = matches.subcommand_matches("init") {
            let output_dir = PathBuf::from_str(matches.value_of("destination").unwrap()).unwrap();
            if !output_dir.exists() {
                fs::create_dir_all(&output_dir).unwrap();
            }

            let mut config = ArchetypeConfig::default();
            config.add_variable("Application Name: ", "name");
            config.add_variable("Author name: ", "author");

            let mut config_file = File::create(output_dir.clone().join("archetype.toml")).unwrap();
            config_file
                .write(toml::ser::to_string_pretty(&config).unwrap().as_bytes())
                .unwrap();

            File::create(output_dir.clone().join("README.md")).expect("Error creating archetype README.md");
            File::create(output_dir.clone().join(".gitignore")).expect("Error creating archetype .gitignore");

            let project_dir = output_dir.clone().join("archetype/{{ name # train_case }}");

            fs::create_dir_all(&project_dir).unwrap();

            let mut project_readme = File::create(project_dir.clone().join("README.md")).expect("Error creating project README.md");
            project_readme.write_all(indoc!(r#"
                Project: {{ name | title_case }}
                Author: {{ author | title_case }}
            "#).as_bytes()).expect("Error writing README.md");
            File::create(project_dir.clone().join(".gitignore")).expect("Error creating project .gitignore");
        }
    }

    const VALID_ANSWER_INPUTS: &str = "\
        \nValid Input Examples:\
        \nkey=value\
        \nkey='value'\
        \nkey=\"value\"\
        \n'key'=\"value\"\
        \n\"key\"='value'\
        \n\"key=value\"\
        \n'key=value'\
        \nkey=\"multiple values\"\
        \n'key'='multiple values'\"\
        \n\"key = 'multiple values'\"\
    ";
}
