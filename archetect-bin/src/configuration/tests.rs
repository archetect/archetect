#[cfg(test)]
mod configuration_tests {
    use crate::configuration::{load_user_config, CONFIGURATION_FILE, DOT_CONFIGURATION_FILE};
    use archetect_core::configuration::Configuration;
    use archetect_core::system::{RootedSystemLayout, SystemLayout};
    use clap::ArgMatches;
    use std::fs;
    use tempfile::TempDir;
    use serial_test::serial;

    struct TestContext {
        temp_dir: TempDir,
        layout: RootedSystemLayout,
    }

    impl TestContext {
        fn new() -> Self {
            let temp_dir = TempDir::new().unwrap();
            let layout = RootedSystemLayout::new(temp_dir.path().to_str().unwrap()).unwrap();
            
            // Create the etc directory
            fs::create_dir_all(layout.etc_dir()).unwrap();
            
            Self { temp_dir, layout }
        }

        fn write_system_config(&self, content: &str) {
            let config_path = self.layout.configuration_path();
            fs::write(&config_path, content).unwrap();
        }

        fn write_local_config(&self, filename: &str, content: &str) {
            let current_dir = std::env::current_dir().unwrap();
            let config_path = current_dir.join(filename);
            fs::write(config_path, content).unwrap();
        }

        fn cleanup_local_config(&self, filename: &str) {
            let current_dir = std::env::current_dir().unwrap();
            let config_path = current_dir.join(filename);
            let _ = fs::remove_file(config_path);
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
        
        let config = load_user_config(&ctx.layout, &args).unwrap();
        
        // Should have default actions
        assert!(!config.actions().is_empty());
        assert!(config.action("default").is_some());
        
        // Default values
        assert!(!config.headless());
        assert!(!config.offline());
    }

    #[test]
    fn test_config_serialization_roundtrip() {
        // Test if the issue is in serialization/deserialization
        use config::{Config, File, FileFormat};
        
        let default_config = Configuration::default();
        println!("Debug: original offline = {}", default_config.offline());
        
        let yaml = default_config.to_yaml();
        println!("Debug: serialized YAML:\n{}", yaml);
        
        let config = Config::builder()
            .add_source(File::from_str(&yaml, FileFormat::Yaml))
            .build()
            .unwrap();
        
        let deserialized: Configuration = config.try_deserialize().unwrap();
        println!("Debug: deserialized offline = {}", deserialized.offline());
        
        // This should work
        assert!(!deserialized.offline());
    }
    
    #[test]
    fn test_config_chain_minimal() {
        // Test the exact same chain as load_user_config but step by step
        use config::{Config, File, FileFormat, Environment};
        
        let ctx = TestContext::new();
        let _args = empty_args();
        
        let default_config_yaml = Configuration::default().to_yaml();
        println!("Debug: step 1 - default YAML:\n{}", default_config_yaml);
        
        // Step 1: Just default config
        let config = Config::builder()
            .add_source(File::from_str(&default_config_yaml, FileFormat::Yaml));
        let result1: Configuration = config.build().unwrap().try_deserialize().unwrap();
        println!("Debug: step 1 result - offline = {}", result1.offline());
        
        // Test if it's an environment variable issue
        let config_with_env = Config::builder()
            .add_source(File::from_str(&default_config_yaml, FileFormat::Yaml))
            .add_source(Environment::with_prefix("ARCHETECT"));
        let result_env: Configuration = config_with_env.build().unwrap().try_deserialize().unwrap();
        println!("Debug: with env result - offline = {}", result_env.offline());
        
        // Step 2: Add system config (non-existent)
        let config = Config::builder()
            .add_source(File::from_str(&default_config_yaml, FileFormat::Yaml))
            .add_source(File::with_name(ctx.layout.configuration_path().as_str()).required(false));
        let result2: Configuration = config.build().unwrap().try_deserialize().unwrap(); 
        println!("Debug: step 2 result - offline = {}", result2.offline());
        
        // Test with absolute paths
        let current_dir = std::env::current_dir().unwrap();
        let dot_config_full_path = current_dir.join(DOT_CONFIGURATION_FILE);
        let config_full_path = current_dir.join(CONFIGURATION_FILE);
        
        println!("Debug: Looking for .archetect at: {}", dot_config_full_path.display());
        println!("Debug: Looking for archetect at: {}", config_full_path.display());
        println!("Debug: .archetect exists: {}", dot_config_full_path.exists());
        println!("Debug: archetect exists: {}", config_full_path.exists());
        
        // Test WITHOUT .archetect file loading
        let config_builder_no_dot = Config::builder()
            .add_source(File::from_str(&default_config_yaml, FileFormat::Yaml))
            .add_source(File::with_name(ctx.layout.configuration_path().as_str()).required(false));
        
        let result_no_dot: Configuration = config_builder_no_dot.build().unwrap().try_deserialize().unwrap();
        println!("Debug: WITHOUT .archetect - offline = {}", result_no_dot.offline());
        
        // Test .archetect file WITHOUT FileFormat specification
        let config_builder_no_format = Config::builder()
            .add_source(File::from_str(&default_config_yaml, FileFormat::Yaml))
            .add_source(File::with_name(ctx.layout.configuration_path().as_str()).required(false))
            .add_source(File::with_name(dot_config_full_path.to_str().unwrap()).required(false));
        
        let result_no_format: Configuration = config_builder_no_format.build().unwrap().try_deserialize().unwrap();
        println!("Debug: .archetect WITHOUT format - offline = {}", result_no_format.offline());
        
        // Test .archetect file alone with absolute path
        let config_builder = Config::builder()
            .add_source(File::from_str(&default_config_yaml, FileFormat::Yaml))
            .add_source(File::with_name(ctx.layout.configuration_path().as_str()).required(false))
            .add_source(File::with_name(dot_config_full_path.to_str().unwrap()).format(FileFormat::Yaml).required(false));
        
        let built_config = config_builder.build().unwrap();
        
        // Let's see what the config actually contains as raw data
        if let Ok(config_map) = built_config.clone().try_deserialize::<std::collections::HashMap<String, config::Value>>() {
            println!("Debug: Raw config map: {:?}", config_map);
            if let Some(offline_value) = config_map.get("offline") {
                println!("Debug: offline value in map: {:?}", offline_value);
            }
        }
        
        let result3a: Configuration = built_config.try_deserialize().unwrap();
        println!("Debug: step 3a result (with absolute .archetect) - offline = {}", result3a.offline());
        
        assert!(!result1.offline());
        assert!(!result2.offline()); 
        assert!(!result3a.offline());
    }

    #[test]
    #[serial]
    fn test_system_config_merging() {
        let ctx = TestContext::new();
        let args = empty_args();
        
        // Create system config that modifies headless setting
        ctx.write_system_config(r#"
headless: true
offline: true
actions:
  custom:
    archetype:
      description: "Custom Action"
      source: "https://example.com/custom.git"
"#);
        
        let config = load_user_config(&ctx.layout, &args).unwrap();
        
        // System config should override defaults
        assert!(config.headless());
        assert!(config.offline());
        
        // Should have both default and custom actions
        assert!(config.action("default").is_some());
        assert!(config.action("custom").is_some());
    }

    #[test]
    #[serial]
    fn test_local_config_overrides_system() {
        let ctx = TestContext::new();
        let args = empty_args();
        
        // Test 1: Only system config
        ctx.write_system_config(r#"
offline: true
"#);
        
        let config = load_user_config(&ctx.layout, &args).unwrap();
        assert!(config.offline()); // Should be true from system config
        
        // Test 2: System + local config  
        ctx.write_local_config(DOT_CONFIGURATION_FILE, r#"
headless: false
"#);
        
        let config = load_user_config(&ctx.layout, &args).unwrap();
        
        // Local config should add to system config
        assert!(!config.headless()); // from local
        assert!(config.offline()); // from system, should be preserved
        
        ctx.cleanup_local_config(DOT_CONFIGURATION_FILE);
    }

    #[test]
    #[serial]
    fn test_project_config_overrides_dot_config() {
        let ctx = TestContext::new();
        let args = empty_args();
        
        // .archetect config
        ctx.write_local_config(DOT_CONFIGURATION_FILE, r#"
headless: true
actions:
  dot_action:
    archetype:
      description: "Dot Action"
      source: "https://example.com/dot.git"
"#);
        
        // archetect config (should have higher precedence)
        ctx.write_local_config(CONFIGURATION_FILE, r#"
headless: false
actions:
  project_action:
    archetype:
      description: "Project Action"
      source: "https://example.com/project.git"
  dot_action:
    archetype:
      description: "Overridden Dot Action"
      source: "https://example.com/overridden-dot.git"
"#);
        
        let config = load_user_config(&ctx.layout, &args).unwrap();
        
        // Project config should override .archetect config
        assert!(!config.headless());
        
        // Actions from both configs should be present, with project overriding dot
        assert!(config.action("dot_action").is_some());
        assert!(config.action("project_action").is_some());
        
        // Verify dot_action was overridden by project config
        if let Some(action) = config.action("dot_action") {
            assert_eq!(action.description(), "Overridden Dot Action");
        }
        
        ctx.cleanup_local_config(DOT_CONFIGURATION_FILE);
        ctx.cleanup_local_config(CONFIGURATION_FILE);
    }

    #[test]
    #[serial]
    fn test_cli_args_override_all() {
        let ctx = TestContext::new();
        
        // System config
        ctx.write_system_config(r#"
headless: true
offline: false
"#);
        
        // Create args with CLI overrides
        use clap::{Arg, Command};
        let cmd = Command::new("test")
            .arg(Arg::new("headless").long("headless").action(clap::ArgAction::SetTrue))
            .arg(Arg::new("offline").long("offline").action(clap::ArgAction::SetTrue));
        let args = cmd.try_get_matches_from(vec!["test", "--headless", "--offline"]).unwrap();
        
        let config = load_user_config(&ctx.layout, &args).unwrap();
        
        // CLI args should override everything
        assert!(config.headless());
        assert!(config.offline());
    }

    #[test]
    #[serial]
    fn test_actions_replacement_behavior() {
        let ctx = TestContext::new();
        let args = empty_args();
        
        // System config with an action
        ctx.write_system_config(r#"
actions:
  test_action:
    archetype:
      description: "System Version"
      source: "https://example.com/system.git"
"#);
        
        // Local config that replaces the action completely
        ctx.write_local_config(DOT_CONFIGURATION_FILE, r#"
actions:
  test_action:
    archetype:
      description: "Local Version"
      source: "https://example.com/local.git"
"#);
        
        let config = load_user_config(&ctx.layout, &args).unwrap();
        
        // The action should be completely replaced, not merged
        if let Some(action) = config.action("test_action") {
            assert_eq!(action.description(), "Local Version");
            // The path from system config should not be present
            // This verifies that actions are replaced, not merged
        }
        
        ctx.cleanup_local_config(DOT_CONFIGURATION_FILE);
    }

    #[test]
    #[serial]
    fn test_non_action_sections_merge() {
        let ctx = TestContext::new();
        let args = empty_args();
        
        // System config with answers
        ctx.write_system_config(r#"
answers:
  system_answer: "system_value"
  shared_answer: "system_shared"
"#);
        
        // Local config with additional answers
        ctx.write_local_config(DOT_CONFIGURATION_FILE, r#"
answers:
  local_answer: "local_value"
  shared_answer: "local_shared"
"#);
        
        let config = load_user_config(&ctx.layout, &args).unwrap();
        
        // Answers should be merged, with local overriding system for conflicts
        let answers = config.answers();
        assert!(answers.contains_key("system_answer"));
        assert!(answers.contains_key("local_answer"));
        assert!(answers.contains_key("shared_answer"));
        
        // Local should override system for shared keys
        if let Some(shared) = answers.get("shared_answer") {
            assert_eq!(shared.to_string(), "local_shared");
        }
        
        ctx.cleanup_local_config(DOT_CONFIGURATION_FILE);
    }

    #[test]
    fn test_config_file_cli_argument() {
        let ctx = TestContext::new();
        
        // Create a custom config file
        let custom_config_path = ctx.temp_dir.path().join("custom.yaml");
        fs::write(&custom_config_path, r#"
headless: true
offline: true
actions:
  cli_action:
    archetype:
      description: "CLI Action"
      source: "https://example.com/cli.git"
"#).unwrap();
        
        // Create args with config file
        use clap::{Arg, Command};
        let cmd = Command::new("test")
            .arg(Arg::new("config-file").long("config-file").value_name("FILE"));
        let args = cmd.try_get_matches_from(vec![
            "test", 
            "--config-file", 
            custom_config_path.to_str().unwrap()
        ]).unwrap();
        
        let config = load_user_config(&ctx.layout, &args).unwrap();
        
        // Custom config should be loaded
        assert!(config.headless());
        assert!(config.offline());
        assert!(config.action("cli_action").is_some());
    }

    #[test]
    #[serial]
    fn test_configuration_precedence_order() {
        let ctx = TestContext::new();
        let args = empty_args();
        
        // System config
        ctx.write_system_config(r#"
headless: true
offline: true
answers:
  level: "system"
  system_only: "system"
"#);
        
        // .archetect config
        ctx.write_local_config(DOT_CONFIGURATION_FILE, r#"
headless: false
answers:
  level: "dot"
  dot_only: "dot"
"#);
        
        // archetect config
        ctx.write_local_config(CONFIGURATION_FILE, r#"
offline: false
answers:
  level: "project"
  project_only: "project"
"#);
        
        let config = load_user_config(&ctx.layout, &args).unwrap();
        
        // Verify precedence: project > dot > system > default
        assert!(!config.headless()); // from .archetect (dot), overriding system
        assert!(!config.offline()); // from archetect (project), overriding system
        
        let answers = config.answers();
        assert_eq!(answers.get("level").unwrap().to_string(), "project");
        assert!(answers.contains_key("system_only"));
        assert!(answers.contains_key("dot_only"));
        assert!(answers.contains_key("project_only"));
        
        ctx.cleanup_local_config(DOT_CONFIGURATION_FILE);
        ctx.cleanup_local_config(CONFIGURATION_FILE);
    }

    #[test]
    #[serial]
    fn test_etc_d_directory_loading() {
        let ctx = TestContext::new();
        let args = empty_args();
        
        // Create etc.d directory
        let etc_d_dir = ctx.layout.etc_d_dir();
        fs::create_dir_all(&etc_d_dir).unwrap();
        
        // Create multiple config files in etc.d with sorted names
        fs::write(etc_d_dir.join("10-first.yaml"), r#"
headless: true
actions:
  first_action:
    archetype:
      description: "First Action"
      source: "https://example.com/first.git"
answers:
  first_answer: "first_value"
"#).unwrap();
        
        fs::write(etc_d_dir.join("20-second.yaml"), r#"
offline: true
actions:
  second_action:
    archetype:
      description: "Second Action"
      source: "https://example.com/second.git"
  first_action:
    archetype:
      description: "Overridden First Action"
      source: "https://example.com/overridden.git"
answers:
  second_answer: "second_value"
  first_answer: "overridden_first"
"#).unwrap();
        
        let config = load_user_config(&ctx.layout, &args).unwrap();
        
        // Check that configuration from etc.d files is loaded
        assert!(config.headless()); // from 10-first.yaml
        assert!(config.offline()); // from 20-second.yaml
        
        // Check that actions are loaded and second file overrides first
        assert!(config.action("first_action").is_some());
        assert!(config.action("second_action").is_some());
        
        // Verify that second file overrode the first
        if let Some(action) = config.action("first_action") {
            assert_eq!(action.description(), "Overridden First Action");
        }
        
        // Check answers merging with later files overriding earlier ones
        let answers = config.answers();
        assert_eq!(answers.get("first_answer").unwrap().to_string(), "overridden_first");
        assert_eq!(answers.get("second_answer").unwrap().to_string(), "second_value");
    }

    #[test]
    #[serial]
    fn test_etc_d_directory_precedence() {
        let ctx = TestContext::new();
        let args = empty_args();
        
        // Create etc.d directory and config
        let etc_d_dir = ctx.layout.etc_d_dir();
        fs::create_dir_all(&etc_d_dir).unwrap();
        
        fs::write(etc_d_dir.join("config.yaml"), r#"
offline: true
"#).unwrap();
        
        let config = load_user_config(&ctx.layout, &args).unwrap();
        
        // Configuration from etc.d should be loaded
        assert!(config.offline()); // from etc.d config
    }

    #[test]
    fn test_etc_d_directory_missing() {
        let ctx = TestContext::new();
        let args = empty_args();
        
        // Don't create etc.d directory - should work fine without it
        let config = load_user_config(&ctx.layout, &args).unwrap();
        
        // Should still have default configuration
        assert!(!config.headless());
        assert!(!config.offline());
        assert!(config.action("default").is_some());
    }

    #[test]
    #[serial]
    fn test_etc_d_file_extensions() {
        let ctx = TestContext::new();
        let args = empty_args();
        
        // Create etc.d directory
        let etc_d_dir = ctx.layout.etc_d_dir();
        fs::create_dir_all(&etc_d_dir).unwrap();
        
        // Create files with different extensions
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
        
        let config = load_user_config(&ctx.layout, &args).unwrap();
        
        // Both .yaml and .yml should be loaded, .txt should be ignored
        assert!(config.headless()); // from .yaml
        assert!(config.offline()); // from .yml
    }

    #[test]
    fn test_minimal_config_loading() {
        use config::{Config, File, FileFormat};
        
        // Test direct config loading without the full chain
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
        
        // Simulate the exact same chain as load_user_config
        let default_yaml = Configuration::default().to_yaml();
        
        let args = empty_args();
        
        let config = Config::builder()
            // 1. Default config
            .add_source(File::from_str(&default_yaml, FileFormat::Yaml))
            // 2. Two nonexistent files
            .add_source(File::with_name("/nonexistent/file1").required(false))
            .add_source(File::with_name("/nonexistent/file2").required(false))
            // 3. etc.d file
            .add_source(File::with_name(etc_d_file.to_str().unwrap()).format(FileFormat::Yaml).required(false));
            
        // Add CLI arguments like the real function does
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
    #[serial]
    fn test_etc_d_directory_structure() {
        let ctx = TestContext::new();
        let args = empty_args();
        
        // Document the expected directory structure:
        // For tests using RootedSystemLayout:
        //   - System config: {root}/etc/archetect.yaml  
        //   - etc.d directory: {root}/etc.d/
        // For real usage with NativeSystemLayout:
        //   - System config: ~/.archetect/archetect.yaml
        //   - etc.d directory: ~/.archetect/etc.d/
        
        // Verify the paths
        let etc_dir = ctx.layout.etc_dir();
        let config_path = ctx.layout.configuration_path();
        let etc_d_dir = ctx.layout.etc_d_dir();
        
        assert!(etc_dir.to_string().ends_with("/etc"));
        assert!(config_path.to_string().ends_with("/etc/archetect.yaml"));
        assert!(etc_d_dir.to_string().ends_with("/etc.d"));
        
        // Create etc.d directory and a config file
        fs::create_dir_all(&etc_d_dir).unwrap();
        fs::write(etc_d_dir.join("test.yaml"), r#"
test_setting: true
answers:
  test_answer: "test_value"
"#).unwrap();
        
        let config = load_user_config(&ctx.layout, &args).unwrap();
        
        // Verify the config was loaded
        let answers = config.answers();
        assert_eq!(answers.get("test_answer").unwrap().to_string(), "test_value");
    }
}