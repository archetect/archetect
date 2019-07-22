#[macro_use]
extern crate clap;

use archetect::{self, AnswerConfig, Archetype, ArchetypeConfig, DirectoryArchetype};
use clap::{App, AppSettings, Arg, SubCommand};
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::str::FromStr;

fn main() {
    let matches = App::new("Archetypal")
        .version(&crate_version!()[..])
        .author("Jimmie Fulton <jimmie.fulton@gmail.com")
        .about("Generates Projects and Files from Archetype Template Directories")
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .subcommand(
            SubCommand::with_name("init")
                .about("Creates a minimal template")
                .arg(
                    Arg::with_name("destination")
                        .takes_value(true)
                        .help("Destination")
                        .default_value("."),
                ),
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

    if let Some(matches) = matches.subcommand_matches("create") {
        let from = PathBuf::from_str(matches.value_of("from").unwrap()).unwrap();
        let destination = PathBuf::from_str(matches.value_of("destination").unwrap()).unwrap();
        let archetype = DirectoryArchetype::new(from).unwrap();
        let answer_config =
            AnswerConfig::load(destination.clone()).unwrap_or_else(|_| AnswerConfig::default());
        println!("{}", answer_config);
        let context = archetype.get_context(&answer_config).unwrap();
        archetype.generate(destination, context).unwrap();
    } else if let Some(matches) = matches.subcommand_matches("init") {
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

        fs::create_dir(output_dir.clone().join("archetype")).unwrap();
    }
}
