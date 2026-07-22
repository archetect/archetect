//! Archetype interface — declarative input contract for external tooling.
//!
//! The `interface` section of `archetype.yaml` describes what prompts and
//! switches an archetype expects, without replacing the Lua scripting
//! engine. Consumed by web portals, MCP agents, and documentation
//! generators to dynamically build input forms.
//!
//! See `archetect-core/specs/archetype-interface.md` for the full spec.

use serde::{Deserialize, Serialize};

// ── Top-level interface ────────────────────────────────────────────

/// Declarative input contract for an archetype.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ArchetypeInterface {
    /// How clients should interact with this archetype.
    /// Defaults to `Interactive` — omitted from serialization when default.
    #[serde(default, skip_serializing_if = "InteractionMode::is_default")]
    pub mode: InteractionMode,

    /// Declared prompts, in display order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub prompts: Vec<InterfacePrompt>,

    /// Declared switches (boolean flags, never prompted for).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub switches: Vec<InterfaceSwitch>,

    /// Optional grouping of prompts for UI layout.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub groups: Option<Vec<InterfaceGroup>>,
}

// ── Interaction mode ───────────────────────────────────────────────

/// Advises clients on how to interact with the archetype.
///
/// Defaults to `Interactive` — the safe choice that always works.
/// Authors opt in to `Batch` when they can guarantee a flat prompt flow
/// with no conditional branching.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum InteractionMode {
    /// All required inputs are declared in the interface. Clients can
    /// render a complete form and submit all answers at once — the Lua
    /// script will not ask for inputs beyond what is declared.
    Batch,
    /// The archetype may have branching, conditional prompts, or dynamic
    /// behaviour. Clients should use the prompt-by-prompt interactive
    /// protocol (`ClientIoHandle`). The interface still declares known
    /// inputs for discoverability. Default when `mode` is omitted.
    #[default]
    Interactive,
}

impl InteractionMode {
    /// True when this is the default variant (used for skip_serializing_if).
    #[allow(clippy::trivially_copy_pass_by_ref)] // serde requires &T
    fn is_default(&self) -> bool {
        matches!(self, InteractionMode::Interactive)
    }
}

// ── Prompt ─────────────────────────────────────────────────────────

/// A single declared prompt in the interface contract.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct InterfacePrompt {
    /// Answer key — must match what the Lua script prompts for.
    pub key: String,

    /// Prompt type.
    #[serde(rename = "type")]
    pub prompt_type: PromptType,

    /// Human-readable label for the prompt.
    pub label: String,

    /// Help text / description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub help: Option<String>,

    /// Placeholder hint for text-like inputs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<String>,

    /// Whether the input is required. Default: true.
    #[serde(default = "default_true")]
    pub required: bool,

    /// Default value (type depends on prompt_type).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<serde_yaml::Value>,

    /// Default selections for multiselect/list prompts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub defaults: Option<Vec<String>>,

    /// Options for select/multiselect prompts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<PromptOption>>,

    /// Minimum value/length/items (context depends on prompt_type).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min: Option<i64>,

    /// Maximum value/length/items (context depends on prompt_type).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<i64>,

    /// Regex validation pattern (text prompts only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub validation: Option<String>,
}

/// Prompt types mirroring `ScriptMessage::PromptFor*` variants.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PromptType {
    Text,
    Int,
    Bool,
    Select,
    Multiselect,
    List,
    Editor,
}

// ── Option (for select / multiselect) ──────────────────────────────

/// An option in a select or multiselect prompt.
///
/// Supports two YAML forms:
/// - Short: plain string (value and label are identical)
/// - Long: `{ value, label, help? }`
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct PromptOption {
    /// The value submitted when this option is chosen.
    pub value: String,
    /// Human-readable display label.
    pub label: String,
    /// Optional per-option help text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub help: Option<String>,
}

impl<'de> Deserialize<'de> for PromptOption {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Raw {
            Short(String),
            Long {
                value: String,
                label: String,
                #[serde(default)]
                help: Option<String>,
            },
        }

        match Raw::deserialize(deserializer)? {
            Raw::Short(s) => Ok(PromptOption {
                label: s.clone(),
                value: s,
                help: None,
            }),
            Raw::Long {
                value,
                label,
                help,
            } => Ok(PromptOption {
                value,
                label,
                help,
            }),
        }
    }
}

// ── Switch ─────────────────────────────────────────────────────────

/// A declared switch (boolean flag, never prompted for).
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct InterfaceSwitch {
    /// Switch name — passed to `archetype.switches.is_enabled()`.
    pub key: String,
    /// Human-readable label. Optional — clients fall back to the key.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Description of what the switch enables.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub help: Option<String>,
    /// Default state. Default: false.
    #[serde(default)]
    pub default: bool,
}

// ── Group ──────────────────────────────────────────────────────────

/// A named group of prompts for UI layout.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct InterfaceGroup {
    /// Display label for the group.
    pub label: String,
    /// Prompt keys belonging to this group, in display order.
    pub keys: Vec<String>,
}

// ── Helpers ────────────────────────────────────────────────────────

fn default_true() -> bool {
    true
}

// ── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use indoc::indoc;

    #[test]
    fn test_parse_full_interface() {
        let yaml = indoc! {r#"
            mode: batch
            prompts:
              - key: project_name
                type: text
                label: "Project Name"
                help: "Used for directory and package name"
                placeholder: "my-project"
                required: true
                validation: "^[a-z][a-z0-9-]*$"
              - key: database
                type: select
                label: "Database"
                options:
                  - value: postgres
                    label: "PostgreSQL"
                    help: "Recommended for production"
                  - value: mysql
                    label: "MySQL"
                  - sqlite
                default: postgres
              - key: features
                type: multiselect
                label: "Optional Features"
                options:
                  - value: auth
                    label: "Authentication"
                  - value: metrics
                    label: "Observability"
                min: 0
              - key: port
                type: int
                label: "Server Port"
                default: 8080
                min: 1024
                max: 65535
              - key: enable_telemetry
                type: bool
                label: "Enable Telemetry"
                default: true
              - key: authors
                type: list
                label: "Authors"
                min: 1
              - key: license_header
                type: editor
                label: "License Header"
            switches:
              - key: with_ci
                label: "Include CI/CD"
                help: "Generates GitHub Actions workflows"
              - key: enterprise
                label: "Enterprise Mode"
                default: true
            groups:
              - label: "Project"
                keys: [project_name, port]
              - label: "Database"
                keys: [database]
              - label: "Features"
                keys: [features, enable_telemetry]
        "#};

        let iface: ArchetypeInterface = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(iface.mode, InteractionMode::Batch);

        // Prompts
        assert_eq!(iface.prompts.len(), 7);

        let p0 = &iface.prompts[0];
        assert_eq!(p0.key, "project_name");
        assert_eq!(p0.prompt_type, PromptType::Text);
        assert_eq!(p0.label, "Project Name");
        assert!(p0.required);
        assert_eq!(p0.validation.as_deref(), Some("^[a-z][a-z0-9-]*$"));

        let p1 = &iface.prompts[1];
        assert_eq!(p1.prompt_type, PromptType::Select);
        let opts = p1.options.as_ref().unwrap();
        assert_eq!(opts.len(), 3);
        assert_eq!(opts[0].value, "postgres");
        assert_eq!(opts[0].label, "PostgreSQL");
        assert_eq!(opts[0].help.as_deref(), Some("Recommended for production"));
        // Short-form option
        assert_eq!(opts[2].value, "sqlite");
        assert_eq!(opts[2].label, "sqlite");

        let p2 = &iface.prompts[2];
        assert_eq!(p2.prompt_type, PromptType::Multiselect);
        assert_eq!(p2.min, Some(0));

        let p3 = &iface.prompts[3];
        assert_eq!(p3.prompt_type, PromptType::Int);
        assert_eq!(p3.min, Some(1024));
        assert_eq!(p3.max, Some(65535));

        let p4 = &iface.prompts[4];
        assert_eq!(p4.prompt_type, PromptType::Bool);

        let p5 = &iface.prompts[5];
        assert_eq!(p5.prompt_type, PromptType::List);

        let p6 = &iface.prompts[6];
        assert_eq!(p6.prompt_type, PromptType::Editor);

        // Switches
        assert_eq!(iface.switches.len(), 2);
        assert_eq!(iface.switches[0].key, "with_ci");
        assert!(!iface.switches[0].default);
        assert!(iface.switches[1].default);

        // Groups
        let groups = iface.groups.as_ref().unwrap();
        assert_eq!(groups.len(), 3);
        assert_eq!(groups[0].label, "Project");
        assert_eq!(groups[0].keys, vec!["project_name", "port"]);
    }

    #[test]
    fn test_parse_minimal_interface() {
        let yaml = indoc! {r#"
            prompts:
              - key: name
                type: text
                label: "Name"
        "#};

        let iface: ArchetypeInterface = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(iface.mode, InteractionMode::Interactive);
        assert_eq!(iface.prompts.len(), 1);
        assert!(iface.switches.is_empty());
        assert!(iface.groups.is_none());
    }

    #[test]
    fn test_switch_label_optional() {
        // The documented minimal switch form: key + help, no label.
        let yaml = indoc! {r#"
            switches:
              - key: ci
                help: "Wire GitHub Actions"
        "#};
        let iface: ArchetypeInterface = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(iface.switches.len(), 1);
        assert_eq!(iface.switches[0].key, "ci");
        assert!(iface.switches[0].label.is_none());
    }

    #[test]
    fn test_default_interface_is_empty() {
        let iface = ArchetypeInterface::default();
        assert_eq!(iface.mode, InteractionMode::Interactive);
        assert!(iface.prompts.is_empty());
        assert!(iface.switches.is_empty());
        assert!(iface.groups.is_none());
    }

    #[test]
    fn test_option_short_form() {
        let yaml = r#"- postgres
- mysql"#;
        let opts: Vec<PromptOption> = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(opts[0].value, "postgres");
        assert_eq!(opts[0].label, "postgres");
        assert!(opts[0].help.is_none());
    }

    #[test]
    fn test_option_long_form() {
        let yaml = indoc! {r#"
            - value: postgres
              label: "PostgreSQL"
              help: "Production-grade"
        "#};
        let opts: Vec<PromptOption> = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(opts[0].value, "postgres");
        assert_eq!(opts[0].label, "PostgreSQL");
        assert_eq!(opts[0].help.as_deref(), Some("Production-grade"));
    }

    #[test]
    fn test_mixed_option_forms() {
        let yaml = indoc! {r#"
            - value: postgres
              label: "PostgreSQL"
            - sqlite
        "#};
        let opts: Vec<PromptOption> = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(opts.len(), 2);
        assert_eq!(opts[0].label, "PostgreSQL");
        assert_eq!(opts[1].value, "sqlite");
        assert_eq!(opts[1].label, "sqlite");
    }

    #[test]
    fn test_interface_serializes_to_json() {
        let iface = ArchetypeInterface {
            mode: InteractionMode::Batch,
            prompts: vec![InterfacePrompt {
                key: "name".to_string(),
                prompt_type: PromptType::Text,
                label: "Name".to_string(),
                help: Some("Your name".to_string()),
                placeholder: None,
                required: true,
                default: None,
                defaults: None,
                options: None,
                min: None,
                max: None,
                validation: None,
            }],
            switches: vec![InterfaceSwitch {
                key: "ci".to_string(),
                label: Some("CI".to_string()),
                help: None,
                default: false,
            }],
            groups: None,
        };

        let json = serde_json::to_string_pretty(&iface).unwrap();
        assert!(json.contains("\"mode\": \"batch\""), "json was: {json}");
        assert!(json.contains("\"type\": \"text\""), "json was: {json}");
        assert!(json.contains("\"key\": \"name\""), "json was: {json}");
        assert!(json.contains("\"key\": \"ci\""), "json was: {json}");
    }

    #[test]
    fn test_interface_in_manifest() {
        let yaml = indoc! {r#"
            description: "Test Archetype"
            requires:
              archetect: "3.0.0"
            interface:
              mode: batch
              prompts:
                - key: project_name
                  type: text
                  label: "Project Name"
              switches:
                - key: with_ci
                  label: "Include CI"
        "#};

        let manifest: crate::manifest::Manifest = serde_yaml::from_str(yaml).unwrap();
        let iface = manifest.interface.as_ref().unwrap();
        assert_eq!(iface.mode, InteractionMode::Batch);
        assert_eq!(iface.prompts.len(), 1);
        assert_eq!(iface.switches.len(), 1);
    }

    #[test]
    fn test_interface_with_groups_and_mixed_options() {
        // Exercises the full shape used by rust-clap-cli-archetype.
        let yaml = indoc! {r#"
            description: "Rust CLI (clap)"
            requires:
              archetect: "3.0.0"
            interface:
              mode: interactive
              prompts:
                - key: org_name
                  type: text
                  label: "Organization Name"
                  help: "Short org identifier"
                  placeholder: "acme"
                  required: true
                - key: suffix_name
                  type: select
                  label: "Project Suffix"
                  options:
                    - value: cli
                      label: "cli"
                  default: cli
                  required: false
                - key: license
                  type: select
                  label: "License"
                  options:
                    - Apache-2.0
                    - MIT
                    - GPL-3.0
                    - BSD-3-Clause
                    - None
                  default: Apache-2.0
                - key: use_github
                  type: bool
                  label: "Publish to GitHub"
                  default: false
                - key: github_visibility
                  type: select
                  label: "Repository Visibility"
                  options:
                    - public
                    - private
                  default: public
              groups:
                - label: "Identity"
                  keys: [org_name, suffix_name]
                - label: "Author"
                  keys: [license]
                - label: "GitHub"
                  keys: [use_github, github_visibility]
        "#};

        let manifest: crate::manifest::Manifest = serde_yaml::from_str(yaml).unwrap();
        let iface = manifest.interface.as_ref().unwrap();

        assert_eq!(iface.mode, InteractionMode::Interactive);
        assert_eq!(iface.prompts.len(), 5);

        // Text prompt
        let org = &iface.prompts[0];
        assert_eq!(org.key, "org_name");
        assert_eq!(org.prompt_type, PromptType::Text);
        assert!(org.required);
        assert_eq!(org.placeholder.as_deref(), Some("acme"));

        // Select with long-form option
        let suffix = &iface.prompts[1];
        assert_eq!(suffix.prompt_type, PromptType::Select);
        assert!(!suffix.required);
        let opts = suffix.options.as_ref().unwrap();
        assert_eq!(opts[0].value, "cli");

        // Select with short-form options
        let license = &iface.prompts[2];
        let opts = license.options.as_ref().unwrap();
        assert_eq!(opts.len(), 5);
        assert_eq!(opts[0].value, "Apache-2.0");
        assert_eq!(opts[0].label, "Apache-2.0"); // short form: value == label

        // Bool prompt with default
        let github = &iface.prompts[3];
        assert_eq!(github.prompt_type, PromptType::Bool);

        // Groups
        let groups = iface.groups.as_ref().unwrap();
        assert_eq!(groups.len(), 3);
        assert_eq!(groups[0].label, "Identity");
        assert_eq!(groups[0].keys, vec!["org_name", "suffix_name"]);

        // Round-trip to JSON
        let json = serde_json::to_string_pretty(iface).unwrap();
        // mode: interactive is the default — skip_serializing_if omits it
        assert!(!json.contains("\"mode\""), "default mode should be omitted from JSON");
        assert!(json.contains("\"Apache-2.0\""));
    }
}
