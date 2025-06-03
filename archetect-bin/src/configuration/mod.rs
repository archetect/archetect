use std::collections::HashMap;

use clap::parser::ValueSource;
use clap::ArgMatches;
use config::{Config, ConfigError, File, FileFormat, Source, Value};
use log::debug;

use archetect_core::configuration::Configuration;
use archetect_core::system::SystemLayout;

pub const CONFIGURATION_FILE: &str = "archetect";
pub const DOT_CONFIGURATION_FILE: &str = ".archetect";

/// Load configuration files from {etc_d_dir}/*.yaml in sorted order
/// For NativeSystemLayout, etc_d_dir is typically ~/.archetect/etc.d
/// For RootedSystemLayout (tests), etc_d_dir is {root}/etc.d
fn load_config_dir_files<L: SystemLayout>(
    mut config: config::ConfigBuilder<config::builder::DefaultState>,
    layout: &L,
) -> Result<config::ConfigBuilder<config::builder::DefaultState>, ConfigError> {
    use std::fs;

    let etc_d_dir = layout.etc_d_dir();
    debug!("Looking for etc.d directory at: {}", etc_d_dir);

    // Check if the etc.d directory exists
    if !etc_d_dir.exists() {
        debug!("etc.d directory does not exist");
        return Ok(config);
    }

    debug!("etc.d directory exists, scanning for YAML files");

    // Read the directory and collect .yaml files
    let entries = match fs::read_dir(&etc_d_dir) {
        Ok(entries) => entries,
        Err(e) => {
            debug!("Failed to read etc.d directory: {}", e);
            return Ok(config); // Directory might not be readable, continue gracefully
        }
    };

    let mut yaml_files = Vec::new();
    for entry in entries {
        if let Ok(entry) = entry {
            let path = entry.path();
            debug!("Found file: {}", path.display());
            if let Some(extension) = path.extension() {
                if extension == "yaml" || extension == "yml" {
                    if let Some(path_str) = path.to_str() {
                        debug!("Adding YAML file: {}", path_str);
                        yaml_files.push(path_str.to_string());
                    }
                }
            }
        }
    }

    // Sort the files to ensure consistent ordering
    yaml_files.sort();
    debug!("Sorted YAML files: {:?}", yaml_files);

    // Add each file as a configuration source
    for yaml_file in &yaml_files {
        debug!("Loading config from: {}", yaml_file);
        // Read and display file contents for debugging
        if let Ok(contents) = std::fs::read_to_string(yaml_file) {
            debug!("File contents of {}:\n{}", yaml_file, contents);
        }
        // Explicitly specify YAML format since the config crate might not auto-detect it properly
        config = config.add_source(File::with_name(yaml_file).format(FileFormat::Yaml).required(true));
    }

    debug!("Finished loading {} etc.d files", yaml_files.len());
    Ok(config)
}

pub fn load_user_config<L: SystemLayout>(layout: &L, args: &ArgMatches) -> Result<Configuration, ConfigError> {
    let default_config_yaml = Configuration::default().to_yaml();
    debug!("Default configuration YAML:\n{}", default_config_yaml);

    let config = Config::builder()
        .add_source(File::from_str(&default_config_yaml, FileFormat::Yaml));
    
    // Debug system config file
    let system_config_path = layout.configuration_path();
    debug!("System config path: {}", system_config_path);
    debug!("System config exists: {}", system_config_path.exists());
    if system_config_path.exists() {
        if let Ok(contents) = std::fs::read_to_string(&system_config_path) {
            debug!("System config contents:\n{}", contents);
        }
    }
    
    let config = config.add_source(File::with_name(system_config_path.as_str()).required(false));

    // Load additional config files from ~/.archetect/etc.d/*.yaml in sorted order
    let config = load_config_dir_files(config, layout)?;

    // Debug local config files and current directory
    let current_dir = std::env::current_dir().unwrap();
    debug!("Current working directory: {}", current_dir.display());
    
    let dot_config_path = current_dir.join(DOT_CONFIGURATION_FILE);
    let config_path = current_dir.join(CONFIGURATION_FILE);
    
    debug!("Checking for .archetect config file at: {}", dot_config_path.display());
    if dot_config_path.exists() {
        if let Ok(contents) = std::fs::read_to_string(&dot_config_path) {
            debug!(".archetect config contents:\n{}", contents);
        }
    }
    
    debug!("Checking for archetect config file at: {}", config_path.display());
    if config_path.exists() {
        if let Ok(contents) = std::fs::read_to_string(&config_path) {
            debug!("archetect config contents:\n{}", contents);
        }
    }

    // Use absolute paths for local config files and only add them if they exist
    // This avoids a bug in the config crate where non-existent files can inject default values
    
    let mut config = config;
    
    // Load .archetect.yaml and .archetect.yml files
    for extension in &[".yaml", ".yml"] {
        let config_file = current_dir.join(format!("{}{}", DOT_CONFIGURATION_FILE, extension));
        if config_file.exists() && config_file.is_file() {
            config = config.add_source(
                File::with_name(config_file.to_str().unwrap())
                    .format(FileFormat::Yaml)
                    .required(true),
            );
        }
    }
    
    // Load archetect.yaml and archetect.yml files
    for extension in &[".yaml", ".yml"] {
        let config_file = current_dir.join(format!("{}{}", CONFIGURATION_FILE, extension));
        if config_file.exists() && config_file.is_file() {
            config = config.add_source(
                File::with_name(config_file.to_str().unwrap())
                    .format(FileFormat::Yaml)
                    .required(true),
            );
        }
    }

    // Merge Config File specified from Command Line
    let config = match args.try_get_one::<String>("config-file") {
        Ok(Some(config_file)) => {
            if let Ok(config_file) = shellexpand::full(config_file) {
                config.add_source(File::with_name(config_file.as_ref()).required(true))
            } else {
                config
            }
        }
        _ => config,
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
    let clap_source = ClapSource::new(args.clone(), mappings);
    // Debug what ClapSource is actually collecting
    if let Ok(clap_values) = clap_source.collect() {
        debug!("ClapSource collected values: {:?}", clap_values);
    }
    let config = config.add_source(clap_source);

    let config = config.build()?;
    let result: Configuration = config.try_deserialize()?;
    debug!("Final config headless: {}", result.headless());
    debug!("Final config offline: {}", result.offline());
    Ok(result)
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
            ArgExtractor::String { .. } => matches
                .try_get_one::<String>(key)
                .ok()
                .flatten()
                .map(|value| value.as_str().into()),
            ArgExtractor::Flag { .. } => {
                // Check if the argument is defined before trying to get its value source
                match matches.try_get_one::<bool>(key) {
                    Ok(_) => {
                        match matches.value_source(key) {
                            None => None,
                            Some(source) => match source {
                                // Only override if explicitly set; don't consider a default as an override
                                ValueSource::CommandLine | ValueSource::EnvVariable => {
                                    Some(matches.get_flag(key).into())
                                }
                                _ => None,
                            },
                        }
                    }
                    Err(_) => None, // Argument doesn't exist
                }
            }
            ArgExtractor::Bool { .. } => {
                // Check if the argument is defined before trying to get its value source
                match matches.try_get_one::<bool>(key) {
                    Ok(_) => {
                        match matches.value_source(key) {
                            None => None,
                            Some(source) => match source {
                                // Only override if explicitly set; don't consider a default as an override
                                ValueSource::CommandLine | ValueSource::EnvVariable => {
                                    matches.try_get_one::<bool>(key).ok().flatten().map(|v| (*v).into())
                                }
                                _ => None,
                            },
                        }
                    }
                    Err(_) => None, // Argument doesn't exist
                }
            }
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

#[cfg(test)]
mod tests;
