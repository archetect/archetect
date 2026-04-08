#[cfg(test)]
mod configuration_tests {
    use crate::configuration::{load_user_config_with_cwd, CONFIGURATION_FILE, DOT_CONFIGURATION_FILE};
    use archetect_core::configuration::Configuration;
    use archetect_core::system::{RootedSystemLayout, SystemLayout};
    use clap::ArgMatches;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    /// Per-test isolated environment. Contains:
    /// - A `RootedSystemLayout` for system config and etc.d (under `temp/system/`)
    /// - A separate `cwd` directory used for project config detection (under `temp/cwd/`)
    ///
    /// Tests should never touch the real working directory or system layout.
    struct TestContext {
        temp_dir: TempDir,
        layout: RootedSystemLayout,
        cwd: PathBuf,
    }

    impl TestContext {
        fn new() -> Self {
            let temp_dir = TempDir::new().unwrap();
            let system_root = temp_dir.path().join("system");
            let cwd = temp_dir.path().join("cwd");
            fs::create_dir_all(&system_root).unwrap();
            fs::create_dir_all(&cwd).unwrap();

            let layout = RootedSystemLayout::new(system_root.to_str().unwrap()).unwrap();
            fs::create_dir_all(layout.etc_dir()).unwrap();

            Self { temp_dir, layout, cwd }
        }

        fn write_system_config(&self, content: &str) {
            let config_path = self.layout.configuration_path();
            fs::write(&config_path, content).unwrap();
        }

        /// Write a project config file (e.g., `.archetect.yaml`) into the test's
        /// isolated cwd. No filesystem pollution outside the tempdir.
        fn write_local_config(&self, filename: &str, content: &str) {
            let config_path = self.cwd.join(filename);
            fs::write(config_path, content).unwrap();
        }

        fn cwd(&self) -> &std::path::Path {
            self.cwd.as_path()
        }
    }

    fn empty_args() -> ArgMatches {
        use clap::{Arg, Command};
        let cmd = Command::new("test")
            .arg(Arg::new("config-file").long("config-file").action(clap::ArgAction::Set))
            .arg(Arg::new("force-update").long("force-update").action(clap::ArgAction::SetTrue))
            .arg(Arg::new("offline").long("offline").action(clap::ArgAction::SetTrue))
            .arg(Arg::new("headless").long("headless").action(clap::ArgAction::SetTrue))
            .arg(Arg::new("local").long("local").action(clap::ArgAction::SetTrue))
            .arg(Arg::new("allow-exec").long("allow-exec").action(clap::ArgAction::Set));
        cmd.try_get_matches_from(vec!["test"]).unwrap()
    }

    #[test]
    fn test_default_configuration_only() {
        let ctx = TestContext::new();
        let args = empty_args();

        let config = load_user_config_with_cwd(&ctx.layout, Some(ctx.cwd()), &args).unwrap();

        // Should have default catalog
        assert!(config.catalog().is_some());

        // Default values
        assert!(!config.headless());
        assert!(!config.offline());
    }

    #[test]
    fn test_config_serialization_roundtrip() {
        use config::{Config, File, FileFormat};

        let default_config = Configuration::default();
        let yaml = default_config.to_yaml();

        let config = Config::builder()
            .add_source(File::from_str(&yaml, FileFormat::Yaml))
            .build()
            .unwrap();

        let deserialized: Configuration = config.try_deserialize().unwrap();
        assert!(!deserialized.offline());
    }

    #[test]
    fn test_config_chain_minimal() {
        use config::{Config, File, FileFormat, Environment};

        let ctx = TestContext::new();
        let _args = empty_args();

        let default_config_yaml = Configuration::default().to_yaml();

        // Step 1: Just default config
        let config = Config::builder()
            .add_source(File::from_str(&default_config_yaml, FileFormat::Yaml));
        let result1: Configuration = config.build().unwrap().try_deserialize().unwrap();

        // Test if it's an environment variable issue
        let config_with_env = Config::builder()
            .add_source(File::from_str(&default_config_yaml, FileFormat::Yaml))
            .add_source(Environment::with_prefix("ARCHETECT"));
        let result_env: Configuration = config_with_env.build().unwrap().try_deserialize().unwrap();

        // Step 2: Add system config (non-existent)
        let config = Config::builder()
            .add_source(File::from_str(&default_config_yaml, FileFormat::Yaml))
            .add_source(File::with_name(ctx.layout.configuration_path().as_str()).required(false));
        let result2: Configuration = config.build().unwrap().try_deserialize().unwrap();

        // Test with absolute paths
        let current_dir = std::env::current_dir().unwrap();
        let dot_config_full_path = current_dir.join(DOT_CONFIGURATION_FILE);

        // Test WITHOUT .archetect file loading
        let config_builder_no_dot = Config::builder()
            .add_source(File::from_str(&default_config_yaml, FileFormat::Yaml))
            .add_source(File::with_name(ctx.layout.configuration_path().as_str()).required(false));

        let result_no_dot: Configuration = config_builder_no_dot.build().unwrap().try_deserialize().unwrap();

        // Test .archetect file WITHOUT FileFormat specification
        let config_builder_no_format = Config::builder()
            .add_source(File::from_str(&default_config_yaml, FileFormat::Yaml))
            .add_source(File::with_name(ctx.layout.configuration_path().as_str()).required(false))
            .add_source(File::with_name(dot_config_full_path.to_str().unwrap()).required(false));

        let _result_no_format: Configuration = config_builder_no_format.build().unwrap().try_deserialize().unwrap();

        // Test .archetect file alone with absolute path
        let config_builder = Config::builder()
            .add_source(File::from_str(&default_config_yaml, FileFormat::Yaml))
            .add_source(File::with_name(ctx.layout.configuration_path().as_str()).required(false))
            .add_source(File::with_name(dot_config_full_path.to_str().unwrap()).format(FileFormat::Yaml).required(false));

        let built_config = config_builder.build().unwrap();
        let result3a: Configuration = built_config.try_deserialize().unwrap();

        assert!(!result1.offline());
        assert!(!result_env.offline());
        assert!(!result2.offline());
        assert!(!result_no_dot.offline());
        assert!(!result3a.offline());
    }

    #[test]
    fn test_system_config_merging() {
        let ctx = TestContext::new();
        let args = empty_args();

        ctx.write_system_config(r#"
headless: true
offline: true
"#);

        let config = load_user_config_with_cwd(&ctx.layout, Some(ctx.cwd()), &args).unwrap();

        // System config should override defaults
        assert!(config.headless());
        assert!(config.offline());
    }

    #[test]
    fn test_local_config_overrides_system() {
        let ctx = TestContext::new();
        let args = empty_args();

        // Test 1: Only system config
        ctx.write_system_config(r#"
offline: true
"#);

        let config = load_user_config_with_cwd(&ctx.layout, Some(ctx.cwd()), &args).unwrap();
        assert!(config.offline());

        // Test 2: System + local config
        ctx.write_local_config(&format!("{}.yaml", DOT_CONFIGURATION_FILE), r#"
headless: false
"#);

        let config = load_user_config_with_cwd(&ctx.layout, Some(ctx.cwd()), &args).unwrap();

        assert!(!config.headless()); // from local
        assert!(config.offline()); // from system, should be preserved
    }

    /// Verify catalog REPLACE semantics: project catalog fully replaces global,
    /// it doesn't merge entries.
    #[test]
    fn test_project_catalog_replaces_global() {
        let ctx = TestContext::new();
        let args = empty_args();

        ctx.write_system_config(r#"
catalog:
  global-only:
    description: "Global Only"
    source: "https://example.com/global.git"
  shared-name:
    description: "Global Version"
    source: "https://example.com/global-shared.git"
"#);

        ctx.write_local_config(".archetect.yaml", r#"
catalog:
  project-only:
    description: "Project Only"
    source: "https://example.com/project.git"
"#);

        let config = load_user_config_with_cwd(&ctx.layout, Some(ctx.cwd()), &args).unwrap();
        let catalog = config.catalog().expect("catalog should be set");

        assert!(!catalog.contains_key("global-only"), "Global catalog should be replaced");
        assert!(!catalog.contains_key("shared-name"), "Global catalog should be replaced");
        assert!(catalog.contains_key("project-only"));
        assert_eq!(catalog.len(), 1);
    }

    /// Without a project config, the global catalog should remain intact.
    #[test]
    fn test_no_project_config_keeps_global_catalog() {
        let ctx = TestContext::new();
        let args = empty_args();

        ctx.write_system_config(r#"
catalog:
  global-entry:
    description: "Global"
    source: "https://example.com/global.git"
"#);

        let config = load_user_config_with_cwd(&ctx.layout, Some(ctx.cwd()), &args).unwrap();
        let catalog = config.catalog().expect("catalog should be set");
        assert!(catalog.contains_key("global-entry"));
    }

    /// In v3, multiple project config variants in the same directory is an error.
    #[test]
    fn test_multiple_project_config_variants_errors() {
        let ctx = TestContext::new();
        let args = empty_args();

        ctx.write_local_config(&format!("{}.yaml", DOT_CONFIGURATION_FILE), r#"
headless: true
"#);

        ctx.write_local_config(&format!("{}.yaml", CONFIGURATION_FILE), r#"
headless: false
"#);

        let result = load_user_config_with_cwd(&ctx.layout, Some(ctx.cwd()), &args);
        let err = result.expect_err("Expected an error when multiple project config variants exist");
        let msg = err.to_string();
        assert!(
            msg.contains("Multiple archetect config files found"),
            "Error message should mention multiple config files: {}", msg
        );
        assert!(msg.contains("archetect.yaml"));
        assert!(msg.contains(".archetect.yaml"));
    }

    #[test]
    fn test_cli_args_override_all() {
        let ctx = TestContext::new();

        ctx.write_system_config(r#"
headless: true
offline: false
"#);

        use clap::{Arg, Command};
        let cmd = Command::new("test")
            .arg(Arg::new("headless").long("headless").action(clap::ArgAction::SetTrue))
            .arg(Arg::new("offline").long("offline").action(clap::ArgAction::SetTrue));
        let args = cmd.try_get_matches_from(vec!["test", "--headless", "--offline"]).unwrap();

        let config = load_user_config_with_cwd(&ctx.layout, Some(ctx.cwd()), &args).unwrap();

        assert!(config.headless());
        assert!(config.offline());
    }

    #[test]
    fn test_non_action_sections_merge() {
        let ctx = TestContext::new();
        let args = empty_args();

        ctx.write_system_config(r#"
answers:
  system_answer: "system_value"
  shared_answer: "system_shared"
"#);

        ctx.write_local_config(&format!("{}.yaml", DOT_CONFIGURATION_FILE), r#"
answers:
  local_answer: "local_value"
  shared_answer: "local_shared"
"#);

        let config = load_user_config_with_cwd(&ctx.layout, Some(ctx.cwd()), &args).unwrap();

        let answers = config.answers();
        assert!(answers.contains_key("system_answer"));
        assert!(answers.contains_key("local_answer"));
        assert!(answers.contains_key("shared_answer"));

        if let Some(shared) = answers.get("shared_answer") {
            assert_eq!(shared.to_string(), "local_shared");
        }
    }

    #[test]
    fn test_config_file_cli_argument() {
        let ctx = TestContext::new();

        let custom_config_path = ctx.temp_dir.path().join("custom.yaml");
        fs::write(&custom_config_path, r#"
headless: true
offline: true
"#).unwrap();

        use clap::{Arg, Command};
        let cmd = Command::new("test")
            .arg(Arg::new("config-file").long("config-file").value_name("FILE"));
        let args = cmd.try_get_matches_from(vec![
            "test",
            "--config-file",
            custom_config_path.to_str().unwrap()
        ]).unwrap();

        let config = load_user_config_with_cwd(&ctx.layout, Some(ctx.cwd()), &args).unwrap();

        assert!(config.headless());
        assert!(config.offline());
    }

    /// Verify the configuration precedence order with a single project file:
    /// CLI args > project config > etc.d > system config > default
    #[test]
    fn test_configuration_precedence_order() {
        let ctx = TestContext::new();
        let args = empty_args();

        ctx.write_system_config(r#"
headless: true
offline: true
answers:
  level: "system"
  system_only: "system"
"#);

        ctx.write_local_config(&format!("{}.yaml", DOT_CONFIGURATION_FILE), r#"
headless: false
answers:
  level: "project"
  project_only: "project"
"#);

        let config = load_user_config_with_cwd(&ctx.layout, Some(ctx.cwd()), &args).unwrap();

        assert!(!config.headless()); // from project
        assert!(config.offline()); // from system

        let answers = config.answers();
        assert_eq!(answers.get("level").unwrap().to_string(), "project");
        assert!(answers.contains_key("system_only"));
        assert!(answers.contains_key("project_only"));
    }

    #[test]
    fn test_etc_d_directory_loading() {
        let ctx = TestContext::new();
        let args = empty_args();

        let etc_d_dir = ctx.layout.etc_d_dir();
        fs::create_dir_all(&etc_d_dir).unwrap();

        fs::write(etc_d_dir.join("10-first.yaml"), r#"
headless: true
answers:
  first_answer: "first_value"
"#).unwrap();

        fs::write(etc_d_dir.join("20-second.yaml"), r#"
offline: true
answers:
  second_answer: "second_value"
  first_answer: "overridden_first"
"#).unwrap();

        let config = load_user_config_with_cwd(&ctx.layout, Some(ctx.cwd()), &args).unwrap();

        assert!(config.headless());
        assert!(config.offline());

        let answers = config.answers();
        assert_eq!(answers.get("first_answer").unwrap().to_string(), "overridden_first");
        assert_eq!(answers.get("second_answer").unwrap().to_string(), "second_value");
    }

    #[test]
    fn test_etc_d_directory_precedence() {
        let ctx = TestContext::new();
        let args = empty_args();

        let etc_d_dir = ctx.layout.etc_d_dir();
        fs::create_dir_all(&etc_d_dir).unwrap();

        fs::write(etc_d_dir.join("config.yaml"), r#"
offline: true
"#).unwrap();

        let config = load_user_config_with_cwd(&ctx.layout, Some(ctx.cwd()), &args).unwrap();
        assert!(config.offline());
    }

    #[test]
    fn test_etc_d_directory_missing() {
        let ctx = TestContext::new();
        let args = empty_args();

        let config = load_user_config_with_cwd(&ctx.layout, Some(ctx.cwd()), &args).unwrap();

        assert!(!config.headless());
        assert!(!config.offline());
        assert!(config.catalog().is_some());
    }

    #[test]
    fn test_etc_d_file_extensions() {
        let ctx = TestContext::new();
        let args = empty_args();

        let etc_d_dir = ctx.layout.etc_d_dir();
        fs::create_dir_all(&etc_d_dir).unwrap();

        fs::write(etc_d_dir.join("config.yaml"), r#"
headless: true
"#).unwrap();

        fs::write(etc_d_dir.join("config.yml"), r#"
offline: true
"#).unwrap();

        // This should be ignored (wrong extension)
        fs::write(etc_d_dir.join("config.txt"), r#"
headless: false
"#).unwrap();

        let config = load_user_config_with_cwd(&ctx.layout, Some(ctx.cwd()), &args).unwrap();

        assert!(config.headless());
        assert!(config.offline());
    }

    #[test]
    fn test_minimal_config_loading() {
        use config::{Config, File, FileFormat};

        let temp_dir = tempfile::TempDir::new().unwrap();
        let config_file = temp_dir.path().join("test.yaml");

        fs::write(&config_file, r#"
headless: true
offline: true
"#).unwrap();

        let default_yaml = Configuration::default().to_yaml();

        let config = Config::builder()
            .add_source(File::from_str(
                &default_yaml,
                FileFormat::Yaml,
            ))
            .add_source(File::with_name(config_file.to_str().unwrap()).format(FileFormat::Yaml).required(true))
            .build()
            .unwrap();

        let result: Configuration = config.try_deserialize().unwrap();

        assert!(result.headless());
        assert!(result.offline());
    }

    #[test]
    fn test_config_chain_step_by_step() {
        use config::{Config, File, FileFormat};

        let temp_dir = tempfile::TempDir::new().unwrap();
        let etc_d_file = temp_dir.path().join("test.yaml");

        fs::write(&etc_d_file, r#"
headless: true
offline: true
"#).unwrap();

        let default_yaml = Configuration::default().to_yaml();
        let args = empty_args();

        let config = Config::builder()
            .add_source(File::from_str(&default_yaml, FileFormat::Yaml))
            .add_source(File::with_name("/nonexistent/file1").required(false))
            .add_source(File::with_name("/nonexistent/file2").required(false))
            .add_source(File::with_name(etc_d_file.to_str().unwrap()).format(FileFormat::Yaml).required(false));

        let mut mappings = std::collections::HashMap::new();
        mappings.insert("headless".into(), crate::configuration::ArgExtractor::Flag { path: "headless".into() });
        mappings.insert("offline".into(), crate::configuration::ArgExtractor::Flag { path: "offline".into() });

        let config = config.add_source(crate::configuration::ClapSource::new(args, mappings));
        let config = config.build().unwrap();
        let result: Configuration = config.try_deserialize().unwrap();

        assert!(result.headless());
        assert!(result.offline());
    }

    #[test]
    fn test_etc_d_directory_structure() {
        let ctx = TestContext::new();
        let args = empty_args();

        let etc_dir = ctx.layout.etc_dir();
        let config_path = ctx.layout.configuration_path();
        let etc_d_dir = ctx.layout.etc_d_dir();

        assert!(etc_dir.to_string().ends_with("/etc"));
        assert!(config_path.to_string().ends_with("/etc/archetect.yaml"));
        assert!(etc_d_dir.to_string().ends_with("/etc.d"));

        fs::create_dir_all(&etc_d_dir).unwrap();
        fs::write(etc_d_dir.join("test.yaml"), r#"
answers:
  test_answer: "test_value"
"#).unwrap();

        let config = load_user_config_with_cwd(&ctx.layout, Some(ctx.cwd()), &args).unwrap();

        let answers = config.answers();
        assert_eq!(answers.get("test_answer").unwrap().to_string(), "test_value");
    }
}
