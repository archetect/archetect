use std::collections::HashMap;

use clap::parser::ValueSource;
use clap::ArgMatches;
use config::{Config, ConfigError, File, FileFormat, Source, Value};

use archetect_core::configuration::Configuration;
use archetect_core::system::SystemLayout;

pub const CONFIGURATION_FILE: &str = "archetect";
pub const DOT_CONFIGURATION_FILE: &str = ".archetect";

pub fn load_user_config<L: SystemLayout>(layout: &L, args: &ArgMatches) -> Result<Configuration, ConfigError> {
    let config = Config::builder()
        .add_source(File::from_str(
            Configuration::default().to_yaml().as_str(),
            FileFormat::Yaml,
        ))
        .add_source(File::with_name(layout.etc_dir().join(CONFIGURATION_FILE).as_str()).required(false));


    let config = config.add_source(File::with_name(DOT_CONFIGURATION_FILE).required(false));
    let config = config.add_source(File::with_name(CONFIGURATION_FILE).required(false));

    // Merge Config File specified from Command Line
    let config = if let Some(config_file) = args.get_one::<String>("config-file") {
        if let Ok(config_file) = shellexpand::full(config_file) {
            let config = config.add_source(File::with_name(config_file.as_ref()).required(true));
            config
        } else {
            config
        }
    } else {
        config
    };

    let mut mappings = HashMap::new();
    mappings.insert(
        "force-update".into(),
        ArgExtractor::Flag {
            path: "updates.force".into(),
        },
    );
    mappings.insert("offline".into(), ArgExtractor::Flag { path: "offline".into() });
    mappings.insert(
        "headless".into(),
        ArgExtractor::Flag {
            path: "headless".into(),
        },
    );
    mappings.insert(
        "local".into(),
        ArgExtractor::Flag {
            path: "locals.enabled".into(),
        },
    );
    mappings.insert(
        "allow-exec".into(),
        ArgExtractor::Bool {
            path: "security.allow_exec".into(),
        },
    );
    let config = config.add_source(ClapSource::new(args.clone(), mappings));

    let config = config.build()?;
    config.try_deserialize()
}

#[derive(Clone, Debug)]
struct ClapSource {
    mappings: HashMap<String, ArgExtractor>,
    matches: ArgMatches,
}

impl ClapSource {
    pub fn new(matches: ArgMatches, keys: HashMap<String, ArgExtractor>) -> ClapSource {
        ClapSource {
            mappings: keys,
            matches,
        }
    }
}

impl Source for ClapSource {
    fn clone_into_box(&self) -> Box<dyn Source + Send + Sync> {
        Box::new((*self).clone())
    }

    fn collect(&self) -> Result<HashMap<String, Value>, ConfigError> {
        let mut results = HashMap::new();
        for (key, extractor) in &self.mappings {
            if let Some(value) = extractor.extract(key, &self.matches) {
                results.insert(extractor.path().to_string(), value);
            }
        }
        Ok(results)
    }
}

#[derive(Clone, Debug)]
enum ArgExtractor {
    #[allow(dead_code)]
    String {
        path: String,
    },
    Bool {
        path: String,
    },
    Flag {
        path: String,
    },
}

impl ArgExtractor {
    fn extract(&self, key: &str, matches: &ArgMatches) -> Option<Value> {
        match self {
            ArgExtractor::String { .. } => {
                if let Some(value) = matches.get_one::<String>(key) {
                    Some(value.as_str().into())
                } else {
                    None
                }
            }
            ArgExtractor::Flag { .. } => match matches.value_source(key) {
                None => None,
                Some(source) => match source {
                    // Only override if explicitly set; don't consider a default as an override
                    ValueSource::CommandLine | ValueSource::EnvVariable => Some(matches.get_flag(key).into()),
                    _ => None,
                },
            },
            ArgExtractor::Bool { .. } => match matches.value_source(key) {
                None => None,
                Some(source) => match source {
                    // Only override if explicitly set; don't consider a default as an override
                    ValueSource::CommandLine | ValueSource::EnvVariable => Some(matches.get_one::<bool>(key).map(|v|*v).into()),
                    _ => None,
                },
            },

        }
    }

    fn path(&self) -> &str {
        match self {
            ArgExtractor::String { path } => path.as_str(),
            ArgExtractor::Flag { path } => path.as_str(),
            ArgExtractor::Bool { path } => path.as_str(),
        }
    }
}
