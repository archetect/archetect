use std::collections::HashSet;

use linked_hash_map::LinkedHashMap;
use log::{debug, info, warn};

use crate::Archetect;
use crate::errors::ArchetectError;
use crate::manifest::{CatalogEntry, Manifest};
use crate::source::SourceCommand;

/// Recursively walks a `Manifest` tree, resolving all catalog entry sources
/// and following them into child manifests.
///
/// "Archetypes all the way down": a catalog entry's `source` may point to
/// another archetype that itself has a `catalog` field — those are walked too.
pub struct PreCacher {
    archetect: Archetect,
    visited: HashSet<String>,
    stats: PreCacheStats,
}

#[derive(Debug, Default, Clone)]
pub struct PreCacheStats {
    /// Sources successfully resolved and pulled.
    pub pulled: usize,
    /// Sources skipped because they were already visited in this run.
    pub skipped: usize,
    /// Sources that failed to resolve or pull.
    pub failed: usize,
    /// Child manifests that were loaded and walked.
    pub manifests_walked: usize,
}

impl PreCacher {
    pub fn new(archetect: Archetect) -> Self {
        PreCacher {
            archetect,
            visited: HashSet::new(),
            stats: PreCacheStats::default(),
        }
    }

    /// Pull all sources reachable from this manifest's catalog tree.
    pub fn pull(mut self, manifest: &Manifest) -> Result<PreCacheStats, ArchetectError> {
        if let Some(entries) = manifest.catalog_entries() {
            self.walk(entries, SourceCommand::Pull)?;
        }
        Ok(self.stats)
    }

    /// Invalidate all sources reachable from this manifest's catalog tree.
    /// Forces a re-fetch on next render.
    pub fn invalidate(mut self, manifest: &Manifest) -> Result<PreCacheStats, ArchetectError> {
        if let Some(entries) = manifest.catalog_entries() {
            self.walk(entries, SourceCommand::Invalidate)?;
        }
        Ok(self.stats)
    }

    fn walk(
        &mut self,
        entries: &LinkedHashMap<String, CatalogEntry>,
        command: SourceCommand,
    ) -> Result<(), ArchetectError> {
        for (name, entry) in entries {
            // Process leaves: resolve the source, then descend into the child manifest if any
            if let Some(ref source) = entry.source {
                self.process_leaf(name, source, command)?;
            }

            // Recurse into groups
            if let Some(ref nested) = entry.catalog {
                self.walk(nested, command)?;
            }
        }
        Ok(())
    }

    fn process_leaf(
        &mut self,
        name: &str,
        source: &str,
        command: SourceCommand,
    ) -> Result<(), ArchetectError> {
        if !self.visited.insert(source.to_string()) {
            debug!("Already visited '{}' ({})", name, source);
            self.stats.skipped += 1;
            return Ok(());
        }

        info!("Pre-caching '{}' from {}", name, source);

        // Resolve the source — this triggers git clone/fetch for remote sources
        let resolved = match self.archetect.new_source(source) {
            Ok(s) => s,
            Err(err) => {
                warn!("Failed to resolve '{}' ({}): {}", name, source, err);
                self.stats.failed += 1;
                return Ok(());
            }
        };

        // Execute the cache command (Pull or Invalidate)
        if let Err(err) = resolved.execute(command) {
            warn!("Failed to {} '{}' ({}): {:?}", command_name(command), name, source, err);
            self.stats.failed += 1;
            return Ok(());
        }

        self.stats.pulled += 1;

        // Try to load the child manifest and recurse into its catalog
        let path = match resolved.path() {
            Ok(p) => p,
            Err(err) => {
                debug!("Could not resolve path for '{}': {}", name, err);
                return Ok(());
            }
        };

        match Manifest::load(path) {
            Ok(child_manifest) => {
                self.stats.manifests_walked += 1;
                if let Some(child_entries) = child_manifest.catalog_entries() {
                    debug!("Recursing into '{}' catalog", name);
                    self.walk(child_entries, command)?;
                }
            }
            Err(err) => {
                // Not every leaf is an archetype with its own manifest — that's fine
                debug!("No manifest at '{}' (or unreadable): {}", name, err);
            }
        }

        Ok(())
    }
}

fn command_name(cmd: SourceCommand) -> &'static str {
    match cmd {
        SourceCommand::Pull => "pull",
        SourceCommand::Invalidate => "invalidate",
        SourceCommand::Delete => "delete",
    }
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
        fs::write(dir.join("archetect.yaml"), yaml).unwrap();
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

    #[test]
    fn test_pre_cache_simple_local_catalog() {
        let (_layout_temp, archetect) = build_archetect();
        let workspace = TempDir::new().unwrap();
        let workspace_path = Utf8PathBuf::from(workspace.path().to_str().unwrap());

        // Build a child archetype
        let child_dir = workspace_path.join("child");
        write_manifest(
            &child_dir,
            indoc! {r#"
                description: "Child"
                requires:
                  archetect: "3.0.0"
            "#},
        );

        // Build a parent archetype with a catalog pointing to the child
        let parent_dir = workspace_path.join("parent");
        let child_path = child_dir.as_str();
        write_manifest(
            &parent_dir,
            &format!(
                indoc! {r#"
                    description: "Parent"
                    requires:
                      archetect: "3.0.0"
                    catalog:
                      one:
                        description: "First"
                        source: "{}"
                "#},
                child_path
            ),
        );

        let manifest = Manifest::load(parent_dir).unwrap();
        let stats = PreCacher::new(archetect).pull(&manifest).unwrap();

        assert_eq!(stats.pulled, 1);
        assert_eq!(stats.skipped, 0);
        assert_eq!(stats.failed, 0);
        // The child manifest is loadable so it counts as walked
        assert_eq!(stats.manifests_walked, 1);
    }

    #[test]
    fn test_pre_cache_recursive_catalog() {
        let (_layout_temp, archetect) = build_archetect();
        let workspace = TempDir::new().unwrap();
        let workspace_path = Utf8PathBuf::from(workspace.path().to_str().unwrap());

        // Leaf archetype
        let leaf_dir = workspace_path.join("leaf");
        write_manifest(
            &leaf_dir,
            indoc! {r#"
                description: "Leaf"
                requires:
                  archetect: "3.0.0"
            "#},
        );

        // Mid-level catalog pointing at the leaf
        let mid_dir = workspace_path.join("mid");
        write_manifest(
            &mid_dir,
            &format!(
                indoc! {r#"
                    description: "Mid"
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

        // Root catalog pointing at the mid catalog
        let root_dir = workspace_path.join("root");
        write_manifest(
            &root_dir,
            &format!(
                indoc! {r#"
                    description: "Root"
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

        let manifest = Manifest::load(root_dir).unwrap();
        let stats = PreCacher::new(archetect).pull(&manifest).unwrap();

        // Both mid and leaf get pulled — recursion all the way down
        assert_eq!(stats.pulled, 2);
        assert_eq!(stats.failed, 0);
        assert_eq!(stats.manifests_walked, 2);
    }

    #[test]
    fn test_pre_cache_dedup() {
        let (_layout_temp, archetect) = build_archetect();
        let workspace = TempDir::new().unwrap();
        let workspace_path = Utf8PathBuf::from(workspace.path().to_str().unwrap());

        let leaf_dir = workspace_path.join("shared");
        write_manifest(
            &leaf_dir,
            indoc! {r#"
                description: "Shared"
                requires:
                  archetect: "3.0.0"
            "#},
        );

        // Root with two entries pointing to the same source
        let root_dir = workspace_path.join("root");
        write_manifest(
            &root_dir,
            &format!(
                indoc! {r#"
                    description: "Root"
                    requires:
                      archetect: "3.0.0"
                    catalog:
                      a:
                        description: "A"
                        source: "{}"
                      b:
                        description: "B"
                        source: "{}"
                "#},
                leaf_dir.as_str(),
                leaf_dir.as_str()
            ),
        );

        let manifest = Manifest::load(root_dir).unwrap();
        let stats = PreCacher::new(archetect).pull(&manifest).unwrap();

        // Only one pull, the second is deduplicated
        assert_eq!(stats.pulled, 1);
        assert_eq!(stats.skipped, 1);
    }

    #[test]
    fn test_pre_cache_groups_recurse() {
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

        let root_dir = workspace_path.join("root");
        write_manifest(
            &root_dir,
            &format!(
                indoc! {r#"
                    description: "Root"
                    requires:
                      archetect: "3.0.0"
                    catalog:
                      services:
                        description: "Services"
                        catalog:
                          one:
                            description: "One"
                            source: "{}"
                "#},
                leaf_dir.as_str()
            ),
        );

        let manifest = Manifest::load(root_dir).unwrap();
        let stats = PreCacher::new(archetect).pull(&manifest).unwrap();

        // Walks through the group and pulls the leaf
        assert_eq!(stats.pulled, 1);
    }
}
