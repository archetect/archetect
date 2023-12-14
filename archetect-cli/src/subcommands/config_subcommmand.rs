use std::fs;

use clap::ArgMatches;
use log::error;

use archetect_core::Archetect;
use archetect_core::configuration::Configuration;
use archetect_core::errors::ArchetectError;
use archetect_inquire::{Editor, InquireError};

pub fn handle_config_subcommand(matches: &ArgMatches, archetect: &Archetect) -> Result<(), ArchetectError> {
    match matches.subcommand() {
        Some(("merged", _args)) => {
            println!("{}", archetect.configuration().to_yaml());
        }
        Some(("defaults", _args)) => {
            println!("{}", Configuration::default().to_yaml());
        }
        Some(("edit", _args)) => {
            let config_file = archetect.layout().configuration_path();
            let contents = if config_file.is_file() {
               fs::read_to_string(config_file)?
            } else {
                Configuration::default().to_yaml()
            };
            let message = format!("Edit {}?", archetect.layout().configuration_path().to_string());
            let prompt = Editor::new(&message)
                .with_predefined_text(&contents)
                ;
            match prompt.prompt_skippable() {
                Ok(contents) => {
                    if let Some(contents) = contents {
                        fs::write(archetect.layout().configuration_path(), contents)?;
                    }
                }
                Err(err) => {
                    match err {
                        InquireError::OperationCanceled | InquireError::OperationInterrupted => {}
                        _ => error!("Error: {}", err),
                    }
                }
            }
        }
        Some((unhandled, _args)) => {
            unimplemented!("'{}' config command not implemented", unhandled);
        }
        None => {}
    }

    Ok(())

}