use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use clap::parser::ValueSource;
use clap::ArgMatches;
use config::{Config, ConfigError, File, FileFormat, Source, Value};
use log::debug;

use archetect_core::configuration::Configuration;
use archetect_core::manifest::CatalogEntry;
use archetect_core::system::SystemLayout;
use linked_hash_map::LinkedHashMap;
use serde::Deserialize;

// Legacy base names — used by tests. New code should use PROJECT_CONFIG_VARIANTS.
#[allow(dead_code)]
pub const CONFIGURATION_FILE: &str = "archetect";
#[allow(dead_code)]
pub const DOT_CONFIGURATION_FILE: &str = ".archetect";

/// All accepted variants of a project-level config file, in priority order.
const PROJECT_CONFIG_VARIANTS: &[&str] = &[
    "archetect.yaml",
    "archetect.yml",
    ".archetect.yaml",
    ".archetect.yml",
];

/// Detect a project-level archetect config file in the given directory.
///
/// Returns:
/// - `Ok(Some(path))` if exactly one variant exists
/// - `Ok(None)` if no variants exist
/// - `Err(ConfigError::Message(...))` if multiple variants exist (ambiguous)
pub fn detect_project_config(cwd: &Path) -> Result<Option<PathBuf>, ConfigError> {
    let mut found: Vec<PathBuf> = Vec::new();
    for variant in PROJECT_CONFIG_VARIANTS {
        let path = cwd.join(variant);
        if path.is_file() {
            found.push(path);
        }
    }

    let mut iter = found.into_iter();
    match (iter.next(), iter.next()) {
        (None, _) => Ok(None),
        (Some(only), None) => Ok(Some(only)),
        (Some(first), Some(second)) => {
            let names: Vec<String> = std::iter::once(first)
                .chain(std::iter::once(second))
                .chain(iter)
                .map(|p| {
                    p.file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_else(|| p.display().to_string())
                })
                .collect();
            Err(ConfigError::Message(format!(
                "Multiple archetect config files found in {}: {}. \
                 Remove all but one to avoid ambiguity.",
                cwd.display(),
                names.join(", ")
            )))
        }
    }
}

/// Minimal struct used to extract just the `catalog` field from a project config file
/// without going through the full Configuration deserialization pipeline.
#[derive(Debug, Deserialize)]
struct ProjectCatalogOnly {
    #[serde(default)]
    catalog: Option<LinkedHashMap<String, CatalogEntry>>,
}

/// Minimal struct used to extract just the `switches` field from a config file.
/// Switches are a flag bag folded per-item across config layers (the config
/// crate would otherwise replace the whole array between sources).
/// See docs/specs/flag-resolution-semantics.md.
#[derive(Debug, Deserialize)]
struct SwitchesOnly {
    #[serde(default)]
    switches: Option<Vec<String>>,
}

/// Parse just the `switches` field from a config file. Returns `None` if the
/// file doesn't declare switches or doesn't exist.
fn parse_switches_field(path: &Path) -> Result<Option<Vec<String>>, ConfigError> {
    if !path.is_file() {
        return Ok(None);
    }
    let contents = std::fs::read_to_string(path)
        .map_err(|e| ConfigError::Foreign(Box::new(e)))?;
    let parsed: SwitchesOnly = serde_yaml::from_str(&contents)
        .map_err(|e| ConfigError::Message(format!(
            "Failed to parse config {}: {}", path.display(), e
        )))?;
    Ok(parsed.switches)
}

/// Fold `switches` declarations from all config sources, lowest precedence
/// first, applying per-item overlay semantics (`name` adds, `name=false`
/// removes). Returns `None` if no source declared switches at all.
fn resolve_switch_layers(sources: &[PathBuf]) -> Result<Option<Vec<String>>, ConfigError> {
    let mut resolved: HashSet<String> = HashSet::new();
    let mut any_declared = false;
    for path in sources {
        if let Some(tokens) = parse_switches_field(path)? {
            any_declared = true;
            archetect_core::flags::overlay_flag_tokens(
                &mut resolved,
                tokens.iter().map(String::as_str),
                "switch",
                &path.display().to_string(),
            )
            .map_err(|e| ConfigError::Message(e.to_string()))?;
        }
    }
    if !any_declared {
        return Ok(None);
    }
    let mut switches: Vec<String> = resolved.into_iter().collect();
    switches.sort();
    Ok(Some(switches))
}

/// Parse just the `catalog` field from a project config file. Returns `None`
/// if the file doesn't have a catalog field. Other parse errors are reported.
pub fn parse_project_catalog(path: &Path) -> Result<Option<LinkedHashMap<String, CatalogEntry>>, ConfigError> {
    let contents = std::fs::read_to_string(path)
        .map_err(|e| ConfigError::Foreign(Box::new(e)))?;
    let parsed: ProjectCatalogOnly = serde_yaml::from_str(&contents)
        .map_err(|e| ConfigError::Message(format!(
            "Failed to parse project config {}: {}", path.display(), e
        )))?;
    Ok(parsed.catalog)
}

/// Load configuration files from {etc_d_dir}/*.yaml in sorted order
/// For NativeSystemLayout, etc_d_dir is typically ~/.archetect/etc.d
/// For RootedSystemLayout (tests), etc_d_dir is {root}/etc.d
fn load_config_dir_files<L: SystemLayout>(
    mut config: config::ConfigBuilder<config::builder::DefaultState>,
    layout: &L,
) -> Result<(config::ConfigBuilder<config::builder::DefaultState>, Vec<PathBuf>), ConfigError> {
    use std::fs;

    let etc_d_dir = layout.etc_d_dir();
    debug!("Looking for etc.d directory at: {}", etc_d_dir);

    // Check if the etc.d directory exists
    if !etc_d_dir.exists() {
        debug!("etc.d directory does not exist");
        return Ok((config, Vec::new()));
    }

    debug!("etc.d directory exists, scanning for YAML files");

    // Read the directory and collect .yaml files
    let entries = match fs::read_dir(&etc_d_dir) {
        Ok(entries) => entries,
        Err(e) => {
            debug!("Failed to read etc.d directory: {}", e);
            return Ok((config, Vec::new())); // Directory might not be readable, continue gracefully
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
    let paths = yaml_files.iter().map(PathBuf::from).collect();
    Ok((config, paths))
}

/// Load user configuration using the current working directory for project config detection.
pub fn load_user_config<L: SystemLayout>(layout: &L, args: &ArgMatches) -> Result<Configuration, ConfigError> {
    let current_dir = std::env::current_dir()
        .map_err(|e| ConfigError::Foreign(Box::new(e)))?;
    load_user_config_with_cwd(layout, Some(&current_dir), args)
}

/// Load user configuration WITHOUT detecting any project-level config.
/// Used by `archetect global` to explicitly bypass `.archetect.yaml` overrides.
pub fn load_global_config<L: SystemLayout>(layout: &L, args: &ArgMatches) -> Result<Configuration, ConfigError> {
    load_user_config_with_cwd(layout, None, args)
}

/// Load user configuration with an explicit working directory.
///
/// - `Some(cwd)` → look for project config in `cwd`
/// - `None` → skip project config detection entirely (used by `archetect global`)
///
/// Tests should use `Some(tempdir_path)` to avoid polluting the workspace.
pub fn load_user_config_with_cwd<L: SystemLayout>(
    layout: &L,
    current_dir: Option<&Path>,
    args: &ArgMatches,
) -> Result<Configuration, ConfigError> {
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

    // Track every file source in precedence order for flag-bag fields
    // (switches) that need per-item folding rather than the config
    // crate's whole-array replacement.
    let mut switch_sources: Vec<PathBuf> = vec![PathBuf::from(system_config_path.as_str())];

    // Load additional config files from ~/.archetect/etc.d/*.yaml in sorted order
    let (config, etc_d_paths) = load_config_dir_files(config, layout)?;
    switch_sources.extend(etc_d_paths);

    if let Some(cwd) = current_dir {
        debug!("Current working directory: {}", cwd.display());
    } else {
        debug!("Project config detection disabled (global mode)");
    }

    // Detect a single project config file in CWD if enabled. Errors if multiple
    // variants coexist (no clever merging — explicit failure is safer).
    let project_config_path = match current_dir {
        Some(cwd) => detect_project_config(cwd)?,
        None => None,
    };
    if let Some(ref path) = project_config_path {
        debug!("Detected project config: {}", path.display());
    } else {
        debug!("No project config in CWD");
    }

    let mut config = config;

    // Layer in the project config file (if any) as a config source. The config
    // crate does field-level merge by default — this gives us the answer-merge
    // semantics we want for free. The catalog field is REPLACED separately
    // below to enforce "project replaces global" semantics.
    if let Some(ref project_path) = project_config_path {
        config = config.add_source(
            File::with_name(&project_path.to_string_lossy())
                .format(FileFormat::Yaml)
                .required(true),
        );
        switch_sources.push(project_path.clone());
    }

    // Merge Config File specified from Command Line
    let config = match args.try_get_one::<String>("config-file") {
        Ok(Some(config_file)) => {
            if let Ok(config_file) = shellexpand::full(config_file) {
                switch_sources.push(PathBuf::from(config_file.as_ref()));
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
        "dry-run".into(),
        ArgExtractor::Flag {
            path: "dry_run".into(),
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
    let mut result: Configuration = config.try_deserialize()?;
    debug!("Final config headless: {}", result.headless());
    debug!("Final config offline: {}", result.offline());

    // Switches are a flag bag: fold declarations per-item across all file
    // sources (lowest precedence first), honoring `name=false` negation.
    // This overrides the config crate's whole-array replacement.
    if let Some(switches) = resolve_switch_layers(&switch_sources)? {
        debug!("Resolved switches across config layers: {:?}", switches);
        result.set_switches(switches);
    }

    // Catalog "replace" semantics: if a project config file exists and declares
    // a `catalog`, it fully replaces the global catalog (not field-merged into it).
    if let Some(ref project_path) = project_config_path {
        if let Some(project_catalog) = parse_project_catalog(project_path)? {
            debug!(
                "Replacing global catalog with project catalog from {}",
                project_path.display()
            );
            result.set_catalog(project_catalog);
        }
    }

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

    fn collect(&self) -> Result<config::Map<String, Value>, ConfigError> {
        let mut results = config::Map::new();
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
