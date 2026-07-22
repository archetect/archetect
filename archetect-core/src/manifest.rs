use std::collections::HashSet;
use std::fs;

use camino::Utf8PathBuf;
use linked_hash_map::LinkedHashMap;
use serde::{Deserialize, Serialize};

use archetect_api::ContextMap;

use crate::archetype::archetype_manifest::requirements::RuntimeRequirements;
use crate::archetype::archetype_manifest::templating::TemplatingConfig;
use crate::errors::ArchetypeError;

/// Manifest file name candidates in priority order.
///
/// `archetype.yaml` is the canonical v3 name — it describes the archetype
/// itself, in contrast to `archetect.yaml` which is the *tool's* config
/// (found in `~/.config/archetect/` and as project-level `.archetect.yaml`
/// overrides). The `archetect.yaml` and `archetect.yml` variants are
/// accepted as aliases for backwards compatibility with early v3 development
/// but should not be used for new archetypes.
pub const MANIFEST_FILE_NAMES: &[&str] = &[
    "archetype.yaml",
    "archetype.yml",
    "archetect.yaml",
    "archetect.yml",
];

/// The REMOVED declared-interface forms. A manifest carrying one is a
/// hard error: the prompts are the interface, and `archetect interface`
/// derives the contract from them (see docs/plans/dynamic-interface.md).
const REMOVED_INTERFACE_FILE_NAMES: &[&str] = &["interface.yaml", "interface.yml"];

/// Unified manifest for both archetypes and catalogs.
///
/// When loaded from an `archetype.yaml` (or the `archetect.yaml` alias),
/// all fields are optional except `description` and `requires`. The presence of
/// a `catalog` field and/or a Lua script file determines runtime behavior.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Manifest {
    #[serde(default)]
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub authors: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub languages: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub frameworks: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default)]
    pub requires: RuntimeRequirements,
    // ── Catalog ──
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub catalog: Option<LinkedHashMap<String, CatalogEntry>>,
    // ── Archetype ──
    #[serde(default)]
    pub templating: TemplatingConfig,
}

/// A recursive catalog entry. Either a leaf (has `source`) or a group (has `catalog`).
///
/// Per the v3 ecosystem design, an entry has two independent flags that control
/// how the consumer treats it:
///
/// - `library: true` — eagerly resolve at archetype load time, stage the
///   resolved archetype's `lib/` and `includes/` directories under this
///   entry's local name, and add them to the consumer's `package.path` and
///   includes search list before any script runs.
/// - `show: false` — hide this entry from `catalog.render()` menus. Still
///   resolvable from a script via `catalog.render("name")`. Useful for
///   private dependencies the script invokes by name.
///
/// The two flags are completely independent — `library: true` does NOT imply
/// `show: false`. A library can also appear in menus if the consumer wants.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CatalogEntry {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Source reference — makes this a leaf (renderable archetype).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// Nested catalog entries — makes this a group (submenu).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub catalog: Option<LinkedHashMap<String, CatalogEntry>>,
    /// Remote archetect-server pointer — makes this a federation root.
    /// The entry behaves as a group whose children are fetched on demand
    /// via `BrowseCatalog`, and renders are dispatched over gRPC.
    /// See `docs/plans/federated-catalog.md`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub server: Option<CatalogEntryServer>,
    /// Pre-configured answers passed to the archetype.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub answers: Option<ContextMap>,
    /// Pre-configured switches.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub switches: Option<HashSet<String>>,
    /// Specific keys to use defaults for.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_defaults: Option<HashSet<String>>,
    /// Use defaults for all prompts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_defaults_all: Option<bool>,
    /// When true, eagerly resolve this entry at archetype load and stage its
    /// `lib/` and `includes/` directories into the consumer's runtime.
    /// Default: false (lazy resolution on use).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub library: bool,
    /// When false, hide this entry from `catalog.render()` menus. The entry
    /// remains resolvable by name from scripts. Default: true (visible).
    #[serde(default = "default_show", skip_serializing_if = "is_default_show")]
    pub show: bool,
}

/// Pointer to a remote archetect server. Only the endpoint is required;
/// TLS settings fall back to the top-level `client.tls` section in
/// `archetect.yaml` when omitted here.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CatalogEntryServer {
    pub endpoint: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tls: Option<CatalogEntryServerTls>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct CatalogEntryServerTls {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ca: Option<std::path::PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_cert: Option<std::path::PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_key: Option<std::path::PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
}

fn default_show() -> bool {
    true
}

#[allow(clippy::trivially_copy_pass_by_ref)] // serde requires &T
fn is_default_show(value: &bool) -> bool {
    *value
}

impl CatalogEntry {
    /// True if this entry has a source (leaf — renders an archetype).
    pub fn is_leaf(&self) -> bool {
        self.source.is_some()
    }

    /// True if this entry has nested catalog entries (group — submenu).
    pub fn is_group(&self) -> bool {
        self.catalog.is_some()
    }

    /// True if this entry points at a remote archetect server. Remote
    /// entries act as groups whose children materialize lazily via
    /// `BrowseCatalog`, and whose renders dispatch over gRPC.
    pub fn is_remote(&self) -> bool {
        self.server.is_some()
    }

    /// Get the display description, falling back to the entry name.
    pub fn display_description(&self, name: &str) -> String {
        self.description
            .clone()
            .unwrap_or_else(|| name.to_string())
    }

    /// Validate that the three "kind" fields (source, catalog, server) are
    /// mutually exclusive. Returns the list of field names supplied when
    /// more than one is present. Empty vec means valid.
    pub fn validate_kind_exclusivity(&self) -> Vec<&'static str> {
        let mut present = Vec::new();
        if self.source.is_some() {
            present.push("source");
        }
        if self.catalog.is_some() {
            present.push("catalog");
        }
        if self.server.is_some() {
            present.push("server");
        }
        if present.len() > 1 {
            present
        } else {
            Vec::new()
        }
    }
}

impl Manifest {
    /// Load a manifest from a directory or file path.
    ///
    /// Searches for manifest files in priority order:
    /// `archetype.yaml` > `archetype.yml` > `archetect.yaml` > `archetect.yml`
    pub fn load<P: Into<Utf8PathBuf>>(path: P) -> Result<Manifest, ArchetypeError> {
        let mut path = path.into();

        if path.is_dir() {
            let mut found = false;
            for candidate in MANIFEST_FILE_NAMES {
                let config_file = path.join(candidate);
                if config_file.exists() {
                    path = config_file;
                    found = true;
                    break;
                }
            }

            if !found {
                return Err(ArchetypeError::ArchetypeConfigMissing);
            }
        }

        if !path.exists() {
            return Err(ArchetypeError::ArchetypeManifestNotFound { path });
        }

        let config = fs::read_to_string(&path)?;
        let mut manifest = serde_yaml::from_str::<Manifest>(&config)
            .map_err(|source| ArchetypeError::ArchetypeManifestSyntaxError { path: path.clone(), source })?;

        // The declared interface was REMOVED after the derived interface
        // shipped (docs/plans/dynamic-interface.md): a declaration is a
        // second copy of what the prompts already say, and second copies
        // drift. Carrying one is a hard error naming the migration.
        if let Ok(raw) = serde_yaml::from_str::<serde_yaml::Value>(&config) {
            if raw.get("interface").is_some() {
                return Err(ArchetypeError::DeclaredInterfaceRemoved {
                    path,
                    form: "an inline `interface:` block".to_string(),
                });
            }
        }
        if let Some(dir) = path.parent() {
            for candidate in REMOVED_INTERFACE_FILE_NAMES {
                if dir.join(candidate).exists() {
                    return Err(ArchetypeError::DeclaredInterfaceRemoved {
                        path,
                        form: format!("a sibling {}", candidate),
                    });
                }
            }
        }

        Ok(manifest)
    }

    /// True if this manifest has catalog entries.
    pub fn has_catalog(&self) -> bool {
        self.catalog
            .as_ref()
            .map(|c| !c.is_empty())
            .unwrap_or(false)
    }

    /// Get the catalog entries, if any.
    pub fn catalog_entries(&self) -> Option<&LinkedHashMap<String, CatalogEntry>> {
        self.catalog.as_ref()
    }

    /// Get metadata for indexing/search.
    pub fn metadata(&self) -> Metadata {
        Metadata {
            description: self.description.clone(),
            summary: self.summary.clone(),
            authors: self.authors.clone(),
            languages: self.languages.clone(),
            frameworks: self.frameworks.clone(),
            tags: self.tags.clone(),
        }
    }
}

/// Extracted metadata for indexing and search.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Metadata {
    pub description: String,
    pub summary: Option<String>,
    pub authors: Vec<String>,
    pub languages: Vec<String>,
    pub frameworks: Vec<String>,
    pub tags: Vec<String>,
}

impl Metadata {
    /// Build a searchable text blob for FTS.
    pub fn searchable_text(&self) -> String {
        let mut text = self.description.to_lowercase();
        if let Some(ref s) = self.summary {
            text.push(' ');
            text.push_str(&s.to_lowercase());
        }
        for v in &self.authors {
            text.push(' ');
            text.push_str(&v.to_lowercase());
        }
        for v in &self.languages {
            text.push(' ');
            text.push_str(&v.to_lowercase());
        }
        for v in &self.frameworks {
            text.push(' ');
            text.push_str(&v.to_lowercase());
        }
        for v in &self.tags {
            text.push(' ');
            text.push_str(&v.to_lowercase());
        }
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use indoc::indoc;

    #[test]
    fn test_parse_catalog_manifest() {
        let yaml = indoc! {r#"
            description: "Acme Platform"
            summary: "Service archetypes for Acme"
            authors: ["Platform Team"]
            languages: ["Rust", "Java"]
            tags: ["microservices"]

            requires:
              archetect: "3.0.0"

            catalog:
              services:
                description: "Backend Services"
                catalog:
                  grpc:
                    description: "gRPC Service"
                    source: "git@github.com:org/rust-grpc.git"
                    answers:
                      framework: Tonic
                    switches: ["with-health-check"]
                  rest:
                    description: "REST Service"
                    source: "git@github.com:org/rust-rest.git"
              frontends:
                description: "Frontend Applications"
                source: "git@github.com:org/catalog-frontends.git"
        "#};

        let manifest: Manifest = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(manifest.description, "Acme Platform");
        assert_eq!(manifest.summary.as_deref(), Some("Service archetypes for Acme"));
        assert_eq!(manifest.authors, vec!["Platform Team"]);
        assert!(manifest.has_catalog());

        let catalog = manifest.catalog.as_ref().unwrap();
        assert_eq!(catalog.len(), 2);

        // services is a group
        let services = &catalog["services"];
        assert!(services.is_group());
        assert!(!services.is_leaf());
        let services_entries = services.catalog.as_ref().unwrap();
        assert_eq!(services_entries.len(), 2);

        // services/grpc is a leaf with answers
        let grpc = &services_entries["grpc"];
        assert!(grpc.is_leaf());
        assert!(!grpc.is_group());
        assert_eq!(grpc.source.as_deref(), Some("git@github.com:org/rust-grpc.git"));
        assert!(grpc.answers.is_some());
        assert!(grpc.switches.is_some());

        // frontends is a delegate (leaf pointing to another archetype)
        let frontends = &catalog["frontends"];
        assert!(frontends.is_leaf());
        assert_eq!(frontends.source.as_deref(), Some("git@github.com:org/catalog-frontends.git"));
    }

    #[test]
    fn test_parse_archetype_manifest() {
        let yaml = indoc! {r#"
            description: "Rust gRPC Service"
            authors: ["Jimmie Fulton"]
            languages: ["Rust"]
            frameworks: ["Tonic"]
            tags: ["service", "grpc"]

            requires:
              archetect: "3.0.0"

            catalog:
              shared-types:
                description: "Shared Types"
                source: "git@github.com:org/shared-types.git"

            scripting:
              main: "archetype.lua"

            templating:
              undefined: strict
        "#};

        let manifest: Manifest = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(manifest.description, "Rust gRPC Service");
        assert!(manifest.has_catalog());
    }

    #[test]
    fn test_parse_hybrid_manifest() {
        let yaml = indoc! {r#"
            description: "Acme Orchestrator"

            requires:
              archetect: "3.0.0"

            catalog:
              rust-services:
                description: "Rust Services"
                catalog:
                  grpc:
                    description: "gRPC"
                    source: "git@github.com:org/grpc.git"
              java-services:
                description: "Java Services"
                source: "git@github.com:org/catalog-java.git"

            scripting:
              main: "archetype.lua"
        "#};

        let manifest: Manifest = serde_yaml::from_str(yaml).unwrap();
        assert!(manifest.has_catalog());
    }

    #[test]
    fn test_legacy_archetype_yaml_compat() {
        // Existing archetype.yaml files have no catalog, summary, etc.
        let yaml = indoc! {r#"
            description: "Simple CLI"
            requires:
              archetect: "2.0.0"
        "#};

        let manifest: Manifest = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(manifest.description, "Simple CLI");
        assert!(!manifest.has_catalog());
        assert!(manifest.authors.is_empty());
    }

    #[test]
    fn test_deeply_nested_catalog() {
        let yaml = indoc! {r#"
            description: "Deep Catalog"
            requires:
              archetect: "3.0.0"
            catalog:
              level1:
                description: "Level 1"
                catalog:
                  level2:
                    description: "Level 2"
                    catalog:
                      leaf:
                        description: "Deep Leaf"
                        source: "git@github.com:org/deep.git"
        "#};

        let manifest: Manifest = serde_yaml::from_str(yaml).unwrap();
        let l1 = &manifest.catalog.as_ref().unwrap()["level1"];
        let l2 = &l1.catalog.as_ref().unwrap()["level2"];
        let leaf = &l2.catalog.as_ref().unwrap()["leaf"];
        assert!(leaf.is_leaf());
        assert_eq!(leaf.description.as_deref(), Some("Deep Leaf"));
    }

    #[test]
    fn test_metadata_searchable_text() {
        let meta = Metadata {
            description: "gRPC Service".to_string(),
            summary: Some("A Rust gRPC microservice".to_string()),
            authors: vec!["Jimmie".to_string()],
            languages: vec!["Rust".to_string()],
            frameworks: vec!["Tonic".to_string()],
            tags: vec!["service".to_string(), "grpc".to_string()],
        };

        let text = meta.searchable_text();
        assert!(text.contains("grpc service"));
        assert!(text.contains("rust"));
        assert!(text.contains("tonic"));
        assert!(text.contains("jimmie"));
    }

    #[test]
    fn test_display_description_fallback() {
        let entry = CatalogEntry {
            description: None,
            source: Some("git@github.com:org/thing.git".to_string()),
            catalog: None,
            answers: None,
            switches: None,
            use_defaults: None,
            use_defaults_all: None,
            server: None,
            library: false,
            show: true,
        };

        assert_eq!(entry.display_description("my-archetype"), "my-archetype");

        let entry_with_desc = CatalogEntry {
            description: Some("My Thing".to_string()),
            ..entry
        };
        assert_eq!(entry_with_desc.display_description("my-archetype"), "My Thing");
    }

    // ---------- v3 ecosystem catalog entry flags ----------

    #[test]
    fn test_catalog_entry_defaults() {
        let yaml = indoc! {r#"
            description: "Test"
            source: "git@github.com:org/thing.git"
        "#};
        let entry: CatalogEntry = serde_yaml::from_str(yaml).unwrap();
        assert!(!entry.library, "library defaults to false");
        assert!(entry.show, "show defaults to true");
    }

    #[test]
    fn test_catalog_entry_with_library_true() {
        let yaml = indoc! {r#"
            source: "git@github.com:org/thing.git"
            library: true
        "#};
        let entry: CatalogEntry = serde_yaml::from_str(yaml).unwrap();
        assert!(entry.library);
        assert!(entry.show, "show is independent of library");
    }

    #[test]
    fn test_catalog_entry_with_show_false() {
        let yaml = indoc! {r#"
            source: "git@github.com:org/thing.git"
            show: false
        "#};
        let entry: CatalogEntry = serde_yaml::from_str(yaml).unwrap();
        assert!(!entry.show);
        assert!(!entry.library, "library is independent of show");
    }

    #[test]
    fn test_catalog_entry_flags_independent() {
        // All four corners of (library, show) are valid and independent.
        let yaml = indoc! {r#"
            source: "git@github.com:org/thing.git"
            library: true
            show: false
        "#};
        let entry: CatalogEntry = serde_yaml::from_str(yaml).unwrap();
        assert!(entry.library);
        assert!(!entry.show);
    }

    #[test]
    fn test_catalog_entry_library_visible_in_menu() {
        // The "show this library in menus too" case.
        let yaml = indoc! {r#"
            source: "git@github.com:org/thing.git"
            library: true
            show: true
        "#};
        let entry: CatalogEntry = serde_yaml::from_str(yaml).unwrap();
        assert!(entry.library);
        assert!(entry.show);
    }

    // ---------- federated catalog (server: field) ----------

    #[test]
    fn test_catalog_entry_server_parses() {
        let yaml = indoc! {r#"
            description: "Acme internal"
            server:
              endpoint: "https://archetect.acme.corp:8443"
              tls:
                ca: "/etc/archetect/acme-ca.crt"
                domain: archetect.acme.corp
        "#};
        let entry: CatalogEntry = serde_yaml::from_str(yaml).unwrap();
        assert!(entry.is_remote(), "entry should be flagged as remote");
        let server = entry.server.as_ref().unwrap();
        assert_eq!(server.endpoint, "https://archetect.acme.corp:8443");
        let tls = server.tls.as_ref().unwrap();
        assert_eq!(
            tls.ca.as_ref().and_then(|p| p.to_str()),
            Some("/etc/archetect/acme-ca.crt")
        );
        assert_eq!(tls.domain.as_deref(), Some("archetect.acme.corp"));
    }

    #[test]
    fn test_catalog_entry_server_without_tls_parses() {
        let yaml = indoc! {r#"
            server:
              endpoint: "http://localhost:8080"
        "#};
        let entry: CatalogEntry = serde_yaml::from_str(yaml).unwrap();
        assert!(entry.is_remote());
        assert!(entry.server.as_ref().unwrap().tls.is_none());
    }

    #[test]
    fn test_validate_kind_exclusivity_source_plus_server() {
        let yaml = indoc! {r#"
            source: "git@github.com:org/thing.git"
            server:
              endpoint: "https://archetect.acme.corp:8443"
        "#};
        let entry: CatalogEntry = serde_yaml::from_str(yaml).unwrap();
        let violations = entry.validate_kind_exclusivity();
        assert_eq!(violations, vec!["source", "server"]);
    }

    #[test]
    fn test_validate_kind_exclusivity_all_three() {
        let yaml = indoc! {r#"
            source: "git@github.com:org/thing.git"
            catalog:
              child:
                source: "git@github.com:org/child.git"
            server:
              endpoint: "https://archetect.acme.corp:8443"
        "#};
        let entry: CatalogEntry = serde_yaml::from_str(yaml).unwrap();
        let violations = entry.validate_kind_exclusivity();
        assert_eq!(violations, vec!["source", "catalog", "server"]);
    }

    #[test]
    fn test_validate_kind_exclusivity_single_is_valid() {
        let yaml = indoc! {r#"
            server:
              endpoint: "http://localhost:8080"
        "#};
        let entry: CatalogEntry = serde_yaml::from_str(yaml).unwrap();
        assert!(entry.validate_kind_exclusivity().is_empty());
    }
}
