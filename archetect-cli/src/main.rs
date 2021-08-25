use std::fs;
use std::path::PathBuf;
use std::str::FromStr;

use clap::{ArgMatches, Shell};
use linked_hash_map::LinkedHashMap;
use log::{error, info, warn};

use archetect_core::{Archetect};
use archetect_core::{self, ArchetectError};
use archetect_core::config::{
    AnswerConfig, AnswerInfo, Catalog, CATALOG_FILE_NAME, CatalogEntry,
};
use archetect_core::input::select_from_catalog;
use archetect_core::source::{Source};

mod cli;
pub mod vendor;

fn main() {
    let matches = cli::get_matches().get_matches();

    cli::configure(&matches);

    match execute(matches) {
        Ok(()) => (),
        Err(error) => error!("{}", error),
    }
}

fn execute(matches: ArgMatches) -> Result<(), ArchetectError> {
    let mut archetect = Archetect::builder()
        .with_offline(matches.is_present("offline"))
        .with_headless(matches.is_present("headless"))
        .build()?;

    let mut answers = LinkedHashMap::new();

    if let Ok(user_answers) = AnswerConfig::load(archetect.layout().answers_config()) {
        for (identifier, answer_info) in user_answers.answers() {
            answers.insert(identifier.to_owned(), answer_info.clone());
        }
    }

    if let Some(matches) = matches.values_of("answer-file") {
        for answer_file in matches {
            match AnswerConfig::load(answer_file) {
                Ok(answer_config) => {
                    for (identifier, answer_info) in answer_config.answers() {
                        answers.insert(identifier.to_owned(), answer_info.clone());
                    }
                }
                Err(cause) => {
                    return Err(ArchetectError::AnswerConfigError {
                        path: answer_file.to_owned(),
                        source: cause,
                    });
                }
            }
        }
    }

    if let Some(matches) = matches.values_of("answer") {
        for (identifier, answer_info) in matches.map(|m| AnswerInfo::parse(m).unwrap()) {
            answers.insert(identifier, answer_info);
        }
    }

    if let Some(matches) = matches.values_of("switches") {
        for switch in matches {
            archetect.enable_switch(switch);
        }
    }

    if let Some(matches) = matches.subcommand_matches("cache") {
        let git_cache = archetect.layout().git_cache_dir();
        if let Some(_sub_matches) = matches.subcommand_matches("clear") {
            fs::remove_dir_all(&git_cache).expect("Error deleting archetect cache");
        }
    }

    if let Some(matches) = matches.subcommand_matches("completions") {
        match matches.subcommand() {
            ("fish", Some(_)) => {
                cli::get_matches().gen_completions_to("archetect", Shell::Fish, &mut std::io::stdout())
            }
            ("bash", Some(_)) => {
                cli::get_matches().gen_completions_to("archetect", Shell::Bash, &mut std::io::stdout())
            }
            ("powershell", Some(_)) => {
                cli::get_matches().gen_completions_to("archetect", Shell::PowerShell, &mut std::io::stdout())
            }
            ("zsh", Some(_)) => cli::get_matches().gen_completions_to("archetect", Shell::Zsh, &mut std::io::stdout()),
            (&_, _) => warn!("Unsupported Shell"),
        }
    }

    if let Some(matches) = matches.subcommand_matches("system") {
        if let Some(matches) = matches.subcommand_matches("layout") {
            match matches.subcommand() {
                ("git", Some(_)) => eprintln!("{}", archetect.layout().git_cache_dir().display()),
                ("http", Some(_)) => eprintln!("{}", archetect.layout().http_cache_dir().display()),
                ("answers", Some(_)) => eprintln!("{}", archetect.layout().answers_config().display()),
                ("catalogs", Some(_)) => eprintln!("{}", archetect.layout().catalog_cache_dir().display()),
                ("config", Some(_)) => eprintln!("{}", archetect.layout().configs_dir().display()),
                _ => eprintln!("{}", archetect.layout()),
            }
        }
    }

    if let Some(matches) = matches.subcommand_matches("render") {
        let source = matches.value_of("source").unwrap();
        let destination = PathBuf::from_str(matches.value_of("destination").unwrap()).unwrap();

        let archetype = archetect.load_archetype(source, None)?;

        if let Ok(answer_config) = AnswerConfig::load(destination.clone()) {
            for (identifier, answer_info) in answer_config.answers() {
                answers.insert(identifier.to_owned(), answer_info.clone());
            }
        }
        archetype.render(&mut archetect, &destination, &answers)?;
    }

    if let Some(matches) = matches.subcommand_matches("catalog") {
        let default_source = archetect.layout().catalog().to_str().map(|s| s.to_owned()).unwrap();
        let source = matches.value_of("source").unwrap_or_else(|| &default_source);
        let source = Source::detect(&archetect, source, None)?;

        let mut catalog_file = source.local_path().to_owned();
        if catalog_file.is_dir() {
            catalog_file.push(CATALOG_FILE_NAME);
        }

        if catalog_file.exists() {
            let catalog_source = Source::detect(&archetect, catalog_file.to_str().unwrap(), None)?;
            let catalog = Catalog::load(source.clone())?;

            let catalog_entry = select_from_catalog(&archetect, &catalog, &catalog_source)?;

            match catalog_entry {
                CatalogEntry::Archetype { description: _, source } => {
                    let destination = PathBuf::from_str(matches.value_of("destination").unwrap()).unwrap();

                    let archetype = archetect.load_archetype(&source, None)?;

                    if let Ok(answer_config) = AnswerConfig::load(destination.clone()) {
                        for (identifier, answer_info) in answer_config.answers() {
                            if !answers.contains_key(identifier) {
                                answers.insert(identifier.to_owned(), answer_info.clone());
                            }
                        }
                    }
                    archetype.render(&mut archetect, &destination, &answers)?;
                    return Ok(());
                }
                _ => unreachable!(),
            }
        } else {
            info!("No catalog file exists at {:?}.", catalog_file);
        }
    }

    Ok(())
}
