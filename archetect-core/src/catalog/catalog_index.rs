use linked_hash_map::LinkedHashMap;

use crate::manifest::{CatalogEntry, Manifest, Metadata};

/// A flattened, searchable index of catalog entries built from a manifest's catalog tree.
#[derive(Clone, Debug)]
pub struct CatalogIndex {
    root: Vec<IndexEntry>,
}

/// A single entry in the catalog index.
#[derive(Clone, Debug)]
pub struct IndexEntry {
    /// Slash-separated path from root, e.g. "services/grpc".
    pub path: String,
    /// The key name of this entry (last segment of path).
    pub name: String,
    /// Display description.
    pub description: String,
    /// Whether this is a group or a leaf.
    pub kind: IndexEntryKind,
    /// Metadata extracted from the entry (if available).
    pub metadata: Option<Metadata>,
    /// Children (for groups).
    pub children: Vec<IndexEntry>,
    /// Source reference (for leaves).
    pub source: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IndexEntryKind {
    Group,
    Leaf,
}

impl CatalogIndex {
    /// Build a CatalogIndex from a manifest.
    pub fn from_manifest(manifest: &Manifest) -> Self {
        let root = match manifest.catalog.as_ref() {
            Some(entries) => build_entries(entries, ""),
            None => Vec::new(),
        };
        CatalogIndex { root }
    }

    /// Build a CatalogIndex from pre-built entries (e.g. from `CatalogIndexer`).
    pub fn from_entries(root: Vec<IndexEntry>) -> Self {
        CatalogIndex { root }
    }

    /// Get the root-level entries.
    pub fn root(&self) -> &[IndexEntry] {
        &self.root
    }

    /// Browse entries at a given path. Empty string returns root entries.
    pub fn browse(&self, path: &str) -> Option<&[IndexEntry]> {
        if path.is_empty() {
            return Some(&self.root);
        }

        let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        let mut current = &self.root;

        for segment in &segments {
            let entry = current.iter().find(|e| e.name == *segment)?;
            current = &entry.children;
        }

        Some(current)
    }

    /// Look up a single entry by path.
    pub fn get(&self, path: &str) -> Option<&IndexEntry> {
        let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        if segments.is_empty() {
            return None;
        }

        let mut current = &self.root;
        for (i, segment) in segments.iter().enumerate() {
            let entry = current.iter().find(|e| e.name == *segment)?;
            if i == segments.len() - 1 {
                return Some(entry);
            }
            current = &entry.children;
        }
        None
    }

    /// Full-text search across all entries. Returns entries whose description,
    /// name, or metadata text contains all query terms (case-insensitive).
    pub fn search(&self, query: &str) -> Vec<&IndexEntry> {
        let terms: Vec<String> = query.to_lowercase().split_whitespace().map(String::from).collect();
        if terms.is_empty() {
            return Vec::new();
        }

        let mut results = Vec::new();
        search_entries(&self.root, &terms, &mut results);
        results
    }

    /// Collect all leaf entries (flattened).
    pub fn all_leaves(&self) -> Vec<&IndexEntry> {
        let mut leaves = Vec::new();
        collect_leaves(&self.root, &mut leaves);
        leaves
    }

    /// Collect all source URLs for pre-caching.
    pub fn all_sources(&self) -> Vec<&str> {
        self.all_leaves()
            .iter()
            .filter_map(|e| e.source.as_deref())
            .collect()
    }
}

fn build_entries(entries: &LinkedHashMap<String, CatalogEntry>, prefix: &str) -> Vec<IndexEntry> {
    entries
        .iter()
        .map(|(name, entry)| {
            let path = if prefix.is_empty() {
                name.clone()
            } else {
                format!("{}/{}", prefix, name)
            };

            let kind = if entry.is_group() {
                IndexEntryKind::Group
            } else {
                IndexEntryKind::Leaf
            };

            let children = match entry.catalog.as_ref() {
                Some(nested) => build_entries(nested, &path),
                None => Vec::new(),
            };

            IndexEntry {
                path,
                name: name.clone(),
                description: entry.display_description(name),
                kind,
                metadata: None,
                children,
                source: entry.source.clone(),
            }
        })
        .collect()
}

fn search_entries<'a>(entries: &'a [IndexEntry], terms: &[String], results: &mut Vec<&'a IndexEntry>) {
    for entry in entries {
        let searchable = build_searchable_text(entry);
        if terms.iter().all(|t| searchable.contains(t)) {
            results.push(entry);
        }
        search_entries(&entry.children, terms, results);
    }
}

fn build_searchable_text(entry: &IndexEntry) -> String {
    let mut text = format!("{} {} {}", entry.name, entry.description, entry.path).to_lowercase();
    if let Some(ref meta) = entry.metadata {
        text.push(' ');
        text.push_str(&meta.searchable_text());
    }
    text
}

fn collect_leaves<'a>(entries: &'a [IndexEntry], leaves: &mut Vec<&'a IndexEntry>) {
    for entry in entries {
        if entry.kind == IndexEntryKind::Leaf {
            leaves.push(entry);
        }
        collect_leaves(&entry.children, leaves);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use indoc::indoc;

    fn test_manifest() -> Manifest {
        let yaml = indoc! {r#"
            description: "Acme Platform"
            requires:
              archetect: "3.0.0"
            catalog:
              services:
                description: "Backend Services"
                catalog:
                  grpc:
                    description: "gRPC Service"
                    source: "git@github.com:org/rust-grpc.git"
                  rest:
                    description: "REST Service"
                    source: "git@github.com:org/rust-rest.git"
              frontends:
                description: "Frontend Applications"
                source: "git@github.com:org/catalog-frontends.git"
              libraries:
                description: "Shared Libraries"
                catalog:
                  core:
                    description: "Core Domain Library"
                    source: "git@github.com:org/lib-core.git"
        "#};
        serde_yaml::from_str(yaml).unwrap()
    }

    #[test]
    fn test_build_index() {
        let manifest = test_manifest();
        let index = CatalogIndex::from_manifest(&manifest);

        assert_eq!(index.root().len(), 3);
        assert_eq!(index.root()[0].name, "services");
        assert_eq!(index.root()[0].kind, IndexEntryKind::Group);
        assert_eq!(index.root()[0].children.len(), 2);
        assert_eq!(index.root()[1].name, "frontends");
        assert_eq!(index.root()[1].kind, IndexEntryKind::Leaf);
        assert_eq!(index.root()[2].name, "libraries");
        assert_eq!(index.root()[2].kind, IndexEntryKind::Group);
    }

    #[test]
    fn test_paths() {
        let manifest = test_manifest();
        let index = CatalogIndex::from_manifest(&manifest);

        let grpc = index.get("services/grpc").unwrap();
        assert_eq!(grpc.path, "services/grpc");
        assert_eq!(grpc.description, "gRPC Service");
        assert_eq!(grpc.kind, IndexEntryKind::Leaf);
        assert_eq!(grpc.source.as_deref(), Some("git@github.com:org/rust-grpc.git"));

        let core = index.get("libraries/core").unwrap();
        assert_eq!(core.path, "libraries/core");
    }

    #[test]
    fn test_browse() {
        let manifest = test_manifest();
        let index = CatalogIndex::from_manifest(&manifest);

        let root = index.browse("").unwrap();
        assert_eq!(root.len(), 3);

        let services = index.browse("services").unwrap();
        assert_eq!(services.len(), 2);
        assert_eq!(services[0].name, "grpc");
        assert_eq!(services[1].name, "rest");
    }

    #[test]
    fn test_search() {
        let manifest = test_manifest();
        let index = CatalogIndex::from_manifest(&manifest);

        let results = index.search("grpc");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "grpc");

        let results = index.search("service");
        assert!(results.len() >= 3); // services group + grpc + rest

        let results = index.search("nonexistent");
        assert!(results.is_empty());
    }

    #[test]
    fn test_all_sources() {
        let manifest = test_manifest();
        let index = CatalogIndex::from_manifest(&manifest);

        let sources = index.all_sources();
        assert_eq!(sources.len(), 4);
        assert!(sources.contains(&"git@github.com:org/rust-grpc.git"));
        assert!(sources.contains(&"git@github.com:org/rust-rest.git"));
        assert!(sources.contains(&"git@github.com:org/catalog-frontends.git"));
        assert!(sources.contains(&"git@github.com:org/lib-core.git"));
    }

    #[test]
    fn test_empty_catalog() {
        let yaml = indoc! {r#"
            description: "Empty"
            requires:
              archetect: "3.0.0"
        "#};
        let manifest: Manifest = serde_yaml::from_str(yaml).unwrap();
        let index = CatalogIndex::from_manifest(&manifest);
        assert!(index.root().is_empty());
        assert!(index.search("anything").is_empty());
        assert!(index.all_sources().is_empty());
    }

    #[test]
    fn test_get_nonexistent() {
        let manifest = test_manifest();
        let index = CatalogIndex::from_manifest(&manifest);
        assert!(index.get("nonexistent").is_none());
        assert!(index.get("services/nonexistent").is_none());
        assert!(index.get("").is_none());
    }
}
