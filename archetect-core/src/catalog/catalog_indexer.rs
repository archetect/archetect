//! Recursive catalog index builder.
//!
//! Walks a catalog tree, resolves leaf sources, and expands sub-catalogs
//! (archetypes whose manifests have a `catalog` field) into a deep
//! `CatalogIndex`. This gives agents full visibility into the entire
//! archetype tree in a single browse or search call.
//!
//! Modeled on `PreCacher`'s walk pattern: dedup by source URL, resilient
//! to resolution failures (log and skip), no panics.

use std::collections::HashSet;

use linked_hash_map::LinkedHashMap;
use log::{debug, info, warn};

use crate::Archetect;
use crate::catalog::catalog_index::{CatalogIndex, IndexEntry, IndexEntryKind};
use crate::manifest::{CatalogEntry, Manifest};

/// Recursively builds a `CatalogIndex` by resolving catalog entry sources
/// and expanding sub-catalogs into the tree.
pub struct CatalogIndexer {
    archetect: Archetect,
    visited: HashSet<String>,
}

impl CatalogIndexer {
    pub fn new(archetect: Archetect) -> Self {
        CatalogIndexer {
            archetect,
            visited: HashSet::new(),
        }
    }

    /// Build a deep `CatalogIndex` from a config catalog.
    ///
    /// For each leaf entry with a source:
    /// - Resolves the source (git clone/fetch for remote sources)
    /// - Loads the child manifest
    /// - If the child has a `catalog` field, expands the leaf into a group
    ///   with those catalog entries as children (recursively)
    /// - Populates metadata from the child manifest
    ///
    /// Failures are logged and skipped — the entry remains a leaf.
    pub fn build_index(mut self, catalog: &LinkedHashMap<String, CatalogEntry>) -> CatalogIndex {
        let entries = self.build_entries(catalog, "");
        CatalogIndex::from_entries(entries)
    }

    fn build_entries(
        &mut self,
        entries: &LinkedHashMap<String, CatalogEntry>,
        prefix: &str,
    ) -> Vec<IndexEntry> {
        entries
            .iter()
            .map(|(name, entry)| self.build_entry(name, entry, prefix))
            .collect()
    }

    fn build_entry(
        &mut self,
        name: &str,
        entry: &CatalogEntry,
        prefix: &str,
    ) -> IndexEntry {
        let path = if prefix.is_empty() {
            name.to_owned()
        } else {
            format!("{}/{}", prefix, name)
        };

        // If the entry already has inline catalog children, build them directly.
        // Inline-declared groups never resolve a source, so they can't be
        // archetypes — just navigation nodes.
        if entry.is_group() {
            let children = match entry.catalog.as_ref() {
                Some(nested) => self.build_entries(nested, &path),
                None => Vec::new(),
            };
            return IndexEntry {
                path,
                name: name.to_owned(),
                description: entry.display_description(name),
                kind: IndexEntryKind::Group,
                metadata: None,
                children,
                source: entry.source.clone(),
                is_archetype: false,
                show: entry.show,
            };
        }

        // Leaf entry — try to resolve its source and classify by what's
        // actually in the resolved tree.
        if let Some(ref source) = entry.source {
            if let Some(expanded) = self.try_expand_source(name, source, &path) {
                // An entry is an archetype iff the resolved source has
                // an archetype.lua file — regardless of whether the
                // manifest also declares catalog entries (those are
                // components intended for composition, not navigation).
                let kind = if expanded.has_script {
                    IndexEntryKind::Leaf
                } else if !expanded.children.is_empty() {
                    IndexEntryKind::Group
                } else {
                    IndexEntryKind::Leaf
                };
                return IndexEntry {
                    path,
                    name: name.to_owned(),
                    description: entry.display_description(name),
                    kind,
                    metadata: Some(expanded.metadata),
                    children: expanded.children,
                    source: Some(source.clone()),
                    is_archetype: expanded.has_script,
                    show: entry.show,
                };
            }
        }

        // Fallback: couldn't resolve source, keep as plain leaf.
        IndexEntry {
            path,
            name: name.to_owned(),
            description: entry.display_description(name),
            kind: IndexEntryKind::Leaf,
            metadata: None,
            children: Vec::new(),
            source: entry.source.clone(),
            is_archetype: false,
            show: entry.show,
        }
    }

    /// Try to resolve a source, load its manifest, and return metadata + child entries.
    /// Returns `None` if resolution fails or the source was already visited.
    fn try_expand_source(
        &mut self,
        name: &str,
        source: &str,
        path_prefix: &str,
    ) -> Option<ExpandedSource> {
        if !self.visited.insert(source.to_owned()) {
            debug!("Already visited '{}' ({}), skipping expansion", name, source);
            return None;
        }

        info!("Indexing '{}' from {}", name, source);

        let resolved = match self.archetect.new_source(source) {
            Ok(s) => s,
            Err(err) => {
                warn!("Failed to resolve '{}' ({}): {}", name, source, err);
                return None;
            }
        };

        let resolved_path = match resolved.path() {
            Ok(p) => p,
            Err(err) => {
                debug!("Could not resolve path for '{}': {}", name, err);
                return None;
            }
        };

        let child_manifest = match Manifest::load(resolved_path.clone()) {
            Ok(m) => m,
            Err(err) => {
                debug!("No manifest at '{}' (or unreadable): {}", name, err);
                return None;
            }
        };

        let metadata = child_manifest.metadata();
        let children = match child_manifest.catalog_entries() {
            Some(child_entries) => {
                debug!("Expanding '{}' — {} catalog entries", name, child_entries.len());
                self.build_entries(child_entries, path_prefix)
            }
            None => Vec::new(),
        };

        // Detect archetype.lua at the resolved source root. If present,
        // this entry is an archetype, not a catalog — even if its
        // manifest also declares catalog entries for composition.
        let has_script = resolved_path.join("archetype.lua").is_file();

        Some(ExpandedSource {
            metadata,
            children,
            has_script,
        })
    }
}

/// Result of resolving a leaf entry's source and inspecting the
/// resulting tree.
struct ExpandedSource {
    metadata: crate::manifest::Metadata,
    children: Vec<IndexEntry>,
    has_script: bool,
}

#[cfg(test)]
mod tests {
    use std::fs;

    use camino::Utf8PathBuf;
    use indoc::indoc;
    use tempfile::TempDir;

    use crate::Archetect;
    use crate::system::RootedSystemLayout;

    use super::*;

    fn write_manifest(dir: &Utf8PathBuf, yaml: &str) {
        fs::create_dir_all(dir).unwrap();
        fs::write(dir.join("archetype.yaml"), yaml).unwrap();
    }

    fn build_archetect() -> (TempDir, Archetect) {
        let temp = TempDir::new().unwrap();
        let layout = RootedSystemLayout::new(temp.path().to_str().unwrap()).unwrap();
        let archetect = Archetect::builder()
            .with_layout(layout)
            .build()
            .unwrap();
        (temp, archetect)
    }

    fn build_config_catalog(entries: Vec<(&str, &str, Option<&str>)>) -> LinkedHashMap<String, CatalogEntry> {
        let mut catalog = LinkedHashMap::new();
        for (name, desc, source) in entries {
            catalog.insert(
                name.to_owned(),
                CatalogEntry {
                    description: Some(desc.to_owned()),
                    source: source.map(|s| s.to_owned()),
                    catalog: None,
                    answers: None,
                    switches: None,
                    use_defaults: None,
                    use_defaults_all: None,
                    library: false,
                    show: true,
                },
            );
        }
        catalog
    }

    #[test]
    fn test_index_simple_leaf() {
        let (_layout_temp, archetect) = build_archetect();
        let workspace = TempDir::new().unwrap();
        let workspace_path = Utf8PathBuf::from(workspace.path().to_str().unwrap());

        let child_dir = workspace_path.join("child");
        write_manifest(
            &child_dir,
            indoc! {r#"
                description: "A Child Archetype"
                requires:
                  archetect: "3.0.0"
            "#},
        );

        let catalog = build_config_catalog(vec![
            ("child", "Child", Some(child_dir.as_str())),
        ]);

        let index = CatalogIndexer::new(archetect).build_index(&catalog);

        assert_eq!(index.root().len(), 1);
        assert_eq!(index.root()[0].name, "child");
        assert_eq!(index.root()[0].kind, IndexEntryKind::Leaf);
        assert!(index.root()[0].metadata.is_some());
    }

    #[test]
    fn test_index_expands_sub_catalog() {
        let (_layout_temp, archetect) = build_archetect();
        let workspace = TempDir::new().unwrap();
        let workspace_path = Utf8PathBuf::from(workspace.path().to_str().unwrap());

        // Leaf archetype
        let leaf_dir = workspace_path.join("leaf");
        write_manifest(
            &leaf_dir,
            indoc! {r#"
                description: "Leaf Archetype"
                requires:
                  archetect: "3.0.0"
            "#},
        );

        // Sub-catalog archetype
        let sub_catalog_dir = workspace_path.join("sub-catalog");
        write_manifest(
            &sub_catalog_dir,
            &format!(
                indoc! {r#"
                    description: "Sub Catalog"
                    requires:
                      archetect: "3.0.0"
                    catalog:
                      leaf:
                        description: "Leaf"
                        source: "{}"
                "#},
                leaf_dir.as_str()
            ),
        );

        let catalog = build_config_catalog(vec![
            ("services", "Backend Services", Some(sub_catalog_dir.as_str())),
        ]);

        let index = CatalogIndexer::new(archetect).build_index(&catalog);

        // "services" should be expanded from leaf to group
        assert_eq!(index.root().len(), 1);
        let services = &index.root()[0];
        assert_eq!(services.name, "services");
        assert_eq!(services.kind, IndexEntryKind::Group);
        assert_eq!(services.children.len(), 1);
        assert_eq!(services.children[0].name, "leaf");
        assert_eq!(services.children[0].path, "services/leaf");
        assert_eq!(services.children[0].kind, IndexEntryKind::Leaf);
    }

    #[test]
    fn test_index_recursive_expansion() {
        let (_layout_temp, archetect) = build_archetect();
        let workspace = TempDir::new().unwrap();
        let workspace_path = Utf8PathBuf::from(workspace.path().to_str().unwrap());

        // Deep leaf
        let leaf_dir = workspace_path.join("leaf");
        write_manifest(
            &leaf_dir,
            indoc! {r#"
                description: "Deep Leaf"
                requires:
                  archetect: "3.0.0"
            "#},
        );

        // Mid-level catalog
        let mid_dir = workspace_path.join("mid");
        write_manifest(
            &mid_dir,
            &format!(
                indoc! {r#"
                    description: "Mid Catalog"
                    requires:
                      archetect: "3.0.0"
                    catalog:
                      deep:
                        description: "Deep"
                        source: "{}"
                "#},
                leaf_dir.as_str()
            ),
        );

        // Top-level catalog
        let top_dir = workspace_path.join("top");
        write_manifest(
            &top_dir,
            &format!(
                indoc! {r#"
                    description: "Top Catalog"
                    requires:
                      archetect: "3.0.0"
                    catalog:
                      mid:
                        description: "Mid"
                        source: "{}"
                "#},
                mid_dir.as_str()
            ),
        );

        let catalog = build_config_catalog(vec![
            ("top", "Top", Some(top_dir.as_str())),
        ]);

        let index = CatalogIndexer::new(archetect).build_index(&catalog);

        // top → mid → deep (3 levels)
        let top = &index.root()[0];
        assert_eq!(top.kind, IndexEntryKind::Group);
        let mid = &top.children[0];
        assert_eq!(mid.name, "mid");
        assert_eq!(mid.kind, IndexEntryKind::Group);
        let deep = &mid.children[0];
        assert_eq!(deep.name, "deep");
        assert_eq!(deep.path, "top/mid/deep");
        assert_eq!(deep.kind, IndexEntryKind::Leaf);
    }

    #[test]
    fn test_index_dedup_sources() {
        let (_layout_temp, archetect) = build_archetect();
        let workspace = TempDir::new().unwrap();
        let workspace_path = Utf8PathBuf::from(workspace.path().to_str().unwrap());

        let shared_dir = workspace_path.join("shared");
        write_manifest(
            &shared_dir,
            indoc! {r#"
                description: "Shared"
                requires:
                  archetect: "3.0.0"
                catalog:
                  inner:
                    description: "Inner"
                    source: "/nonexistent"
            "#},
        );

        // Two config entries pointing to the same source
        let mut catalog = LinkedHashMap::new();
        catalog.insert(
            "a".to_owned(),
            CatalogEntry {
                description: Some("A".to_owned()),
                source: Some(shared_dir.as_str().to_owned()),
                catalog: None,
                answers: None,
                switches: None,
                use_defaults: None,
                use_defaults_all: None,
                library: false,
                show: true,
            },
        );
        catalog.insert(
            "b".to_owned(),
            CatalogEntry {
                description: Some("B".to_owned()),
                source: Some(shared_dir.as_str().to_owned()),
                catalog: None,
                answers: None,
                switches: None,
                use_defaults: None,
                use_defaults_all: None,
                library: false,
                show: true,
            },
        );

        let index = CatalogIndexer::new(archetect).build_index(&catalog);

        // First entry gets expanded, second is deduped (remains a leaf)
        assert_eq!(index.root().len(), 2);
        let a = &index.root()[0];
        assert_eq!(a.name, "a");
        assert_eq!(a.kind, IndexEntryKind::Group);
        assert_eq!(a.children.len(), 1);

        let b = &index.root()[1];
        assert_eq!(b.name, "b");
        // Deduped — no expansion
        assert_eq!(b.kind, IndexEntryKind::Leaf);
    }

    #[test]
    fn test_index_failed_resolution_stays_leaf() {
        let (_layout_temp, archetect) = build_archetect();

        let catalog = build_config_catalog(vec![
            ("broken", "Broken", Some("/nonexistent/path/to/archetype")),
        ]);

        let index = CatalogIndexer::new(archetect).build_index(&catalog);

        assert_eq!(index.root().len(), 1);
        assert_eq!(index.root()[0].name, "broken");
        assert_eq!(index.root()[0].kind, IndexEntryKind::Leaf);
        assert!(index.root()[0].metadata.is_none());
    }

    #[test]
    fn test_index_inline_groups_preserved() {
        let (_layout_temp, archetect) = build_archetect();
        let workspace = TempDir::new().unwrap();
        let workspace_path = Utf8PathBuf::from(workspace.path().to_str().unwrap());

        let leaf_dir = workspace_path.join("leaf");
        write_manifest(
            &leaf_dir,
            indoc! {r#"
                description: "Leaf"
                requires:
                  archetect: "3.0.0"
            "#},
        );

        // Config catalog with inline group structure
        let mut inner = LinkedHashMap::new();
        inner.insert(
            "grpc".to_owned(),
            CatalogEntry {
                description: Some("gRPC Service".to_owned()),
                source: Some(leaf_dir.as_str().to_owned()),
                catalog: None,
                answers: None,
                switches: None,
                use_defaults: None,
                use_defaults_all: None,
                library: false,
                show: true,
            },
        );

        let mut catalog = LinkedHashMap::new();
        catalog.insert(
            "services".to_owned(),
            CatalogEntry {
                description: Some("Backend Services".to_owned()),
                source: None,
                catalog: Some(inner),
                answers: None,
                switches: None,
                use_defaults: None,
                use_defaults_all: None,
                library: false,
                show: true,
            },
        );

        let index = CatalogIndexer::new(archetect).build_index(&catalog);

        let services = &index.root()[0];
        assert_eq!(services.name, "services");
        assert_eq!(services.kind, IndexEntryKind::Group);
        assert_eq!(services.children.len(), 1);
        assert_eq!(services.children[0].name, "grpc");
        assert_eq!(services.children[0].path, "services/grpc");
    }

    #[test]
    fn test_index_searchable_after_build() {
        let (_layout_temp, archetect) = build_archetect();
        let workspace = TempDir::new().unwrap();
        let workspace_path = Utf8PathBuf::from(workspace.path().to_str().unwrap());

        let grpc_dir = workspace_path.join("grpc");
        write_manifest(
            &grpc_dir,
            indoc! {r#"
                description: "gRPC Service Generator"
                languages: ["Rust"]
                frameworks: ["Tonic"]
                requires:
                  archetect: "3.0.0"
            "#},
        );

        let catalog = build_config_catalog(vec![
            ("grpc", "gRPC Service", Some(grpc_dir.as_str())),
        ]);

        let index = CatalogIndexer::new(archetect).build_index(&catalog);

        // Search by name
        let results = index.search("grpc");
        assert_eq!(results.len(), 1);

        // Search by metadata (language from child manifest)
        let results = index.search("rust");
        assert_eq!(results.len(), 1);

        // Search by metadata (framework)
        let results = index.search("tonic");
        assert_eq!(results.len(), 1);
    }
}
