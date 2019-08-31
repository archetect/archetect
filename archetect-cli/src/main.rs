mod cli;

use archetect::config::{
    Answer, AnswerConfig, ArchetypeConfig, CatalogConfig, CatalogConfigError, Variable,
};
use archetect::system::SystemError;
use archetect::util::SourceError;
use archetect::{self, ArchetypeError, ArchetectError};
use clap::{ArgMatches, Shell};
use indoc::indoc;
use log::{error, info, warn};
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
    let matches = cli::get_matches().get_matches();

    cli::configure(&matches);

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

    if let Some(matches) = matches.subcommand_matches("completions") {
        match matches.subcommand() {
            ("fish", Some(_)) => cli::get_matches().gen_completions_to("archetect", Shell::Fish, &mut std::io::stdout()),
            ("bash", Some(_)) => cli::get_matches().gen_completions_to("archetect", Shell::Bash, &mut std::io::stdout()),
            ("powershell", Some(_)) => cli::get_matches().gen_completions_to("archetect", Shell::PowerShell, &mut std::io::stdout()),
            ("zsh", Some(_)) => cli::get_matches().gen_completions_to("archetect", Shell::Zsh, &mut std::io::stdout()),
            (&_, _) => warn!("Unsupported Shell"),
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
    } else if let Some(matches) = matches.subcommand_matches("contents") {
        if let Some(matches) = matches.subcommand_matches("init") {
            let output_dir = PathBuf::from_str(matches.value_of("destination").unwrap()).unwrap();
            if !output_dir.exists() {
                fs::create_dir_all(&output_dir).unwrap();
            }

            let mut config = ArchetypeConfig::default();
            config.add_variable(Variable::with_name("name").with_prompt("Application Name: "));
            config.add_variable(Variable::with_name("author").with_prompt("Author name: "));

            let mut config_file = File::create(output_dir.clone().join("contents.toml")).unwrap();
            config_file
                .write(toml::ser::to_string_pretty(&config).unwrap().as_bytes())
                .unwrap();

            File::create(output_dir.clone().join("README.md")).expect("Error creating contents README.md");
            File::create(output_dir.clone().join(".gitignore")).expect("Error creating contents .gitignore");

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
        SourceError::SourceUnsupported(source) => error!("\"{}\" is not a supported contents path", source),
        SourceError::SourceInvalidPath(source) => error!("\"{}\" is not a valid contents path", source),
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

