//! Library staging for catalog-driven dependencies.
//!
//! When an archetype's manifest declares a catalog entry with `library: true`,
//! the entry is *eagerly* resolved at archetype load time and its `lib/` and
//! `includes/` directories are mounted under the consumer-chosen namespace
//! into a synthetic staging area. The Lua runtime sees these staged
//! directories on `package.path`, and the include resolver sees them in its
//! search list, so:
//!
//! ```lua
//! local casing = require("inflect-helpers.casing")
//! ```
//!
//! ```text
//! {% include "inflect-helpers/header.atl" %}
//! ```
//!
//! ...both find content from a remote library archetype with no extra
//! ceremony.
//!
//! # Layout
//!
//! For each render, the staging dir lives under the archetect cache:
//!
//! ```text
//! <cache>/staging/<consumer-id>/lib/<entry-name>/      → symlink to library's lib/
//! <cache>/staging/<consumer-id>/includes/<entry-name>/ → symlink to library's includes/
//! ```
//!
//! The `<consumer-id>` is a stable hash of the consumer archetype's absolute
//! root path, so concurrent renders of the same consumer share staging and
//! two different consumers don't collide.
//!
//! # Invalidation
//!
//! The consumer's staging dir is **cleared and recreated at the start of
//! every render**. Symlinks are essentially free to recreate, copies (the
//! Windows fallback) cost milliseconds. Treating regenerate-on-render as
//! the steady-state behavior removes any "is staging stale" question — a
//! freshly-pulled library source is automatically reflected on the next
//! render because the symlink/copy is rebuilt.
//!
//! # Symlinks vs copies
//!
//! On Unix-like systems, library `lib/` and `includes/` are symlinked into
//! the staging dirs. On Windows, where symlinks require admin or developer
//! mode, the staging code falls back to copying the directories. The cost
//! is one-time disk-space duplication per render. Acceptable.

use std::collections::hash_map::DefaultHasher;
use std::collections::HashSet;
use std::fs;
use std::hash::{Hash, Hasher};

use camino::{Utf8Path, Utf8PathBuf};
use linked_hash_map::LinkedHashMap;
use log::{debug, warn};

use crate::manifest::{CatalogEntry, Manifest};
use crate::Archetect;

/// Errors that can occur while staging libraries from catalog entries.
#[derive(Debug)]
pub enum LibraryStagingError {
    /// The configured staging root could not be created.
    CacheUnavailable(String),
    /// A library entry's source could not be resolved.
    SourceResolution { entry: String, detail: String },
    /// A library entry resolved but its on-disk path was not accessible.
    PathResolution { entry: String, detail: String },
    /// A symlink (or copy) operation failed.
    StagingIo { entry: String, detail: String },
}

impl std::fmt::Display for LibraryStagingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CacheUnavailable(detail) => write!(f, "library staging cache unavailable: {}", detail),
            Self::SourceResolution { entry, detail } => {
                write!(f, "could not resolve library entry `{}`: {}", entry, detail)
            }
            Self::PathResolution { entry, detail } => {
                write!(f, "could not resolve cached path for library entry `{}`: {}", entry, detail)
            }
            Self::StagingIo { entry, detail } => {
                write!(f, "staging IO failure for library entry `{}`: {}", entry, detail)
            }
        }
    }
}

impl std::error::Error for LibraryStagingError {}

/// A library entry that has been resolved and staged into the consumer's
/// runtime workspace. The staged paths are what get added to `package.path`
/// and the include resolver search list.
#[derive(Debug, Clone)]
pub struct StagedLibrary {
    /// The consumer's local name for this library (catalog map key).
    pub name: String,

    /// The original catalog entry's source string (git URL or local path).
    /// Useful for diagnostics and cache key reconstruction.
    pub source: String,

    /// Path to the staged `lib/` namespace dir, if the resolved library
    /// has a `lib/` directory. `<staging>/lib/<name>/` mirrors the library's
    /// own `lib/` so `require("<name>.module")` resolves correctly.
    pub lib_dir: Option<Utf8PathBuf>,

    /// Path to the staged `includes/` namespace dir, if the resolved
    /// library has an `includes/` directory. `<staging>/includes/<name>/`
    /// mirrors the library's own `includes/` so
    /// `{% include "<name>/template" %}` resolves correctly.
    pub includes_dir: Option<Utf8PathBuf>,

    /// The resolved on-disk root of the library archetype (the directory
    /// that contains `lib/`, `includes/`, `contents/`, etc.). Used by the
    /// Lua `directory.render` implementation to resolve template paths
    /// against the currently-executing archetype's own root rather than
    /// the consumer's root.
    pub source_root: Utf8PathBuf,
}

/// Stages library catalog entries for a single consumer archetype.
///
/// Construct one stager per render, call `stage` once with the consumer's
/// catalog entries, and consume the resulting `Vec<StagedLibrary>` for
/// `package.path` and include-resolver wiring.
pub struct LibraryStager {
    archetect: Archetect,
    /// The consumer archetype's root directory. Used both to scope the
    /// staging dir and to resolve relative `source:` paths in catalog
    /// entries against the consumer's location (not the process CWD).
    consumer_root: Utf8PathBuf,
    /// Hash-derived ID identifying the consumer. Used to scope the staging
    /// dir so concurrent renders of different consumers don't collide.
    consumer_id: String,
}

impl LibraryStager {
    /// Build a stager for a consumer archetype rooted at `consumer_root`.
    /// The consumer ID is derived from the canonical absolute path so
    /// concurrent renders of the same consumer share staging.
    ///
    /// `consumer_root` is canonicalized at construction time. This is
    /// important because Unix symlinks store the target path verbatim and
    /// resolve it relative to the symlink's parent at lookup. If we
    /// symlinked from a relative source path, the resulting symlink would
    /// be broken.
    pub fn new(archetect: Archetect, consumer_root: &Utf8Path) -> Self {
        // Best-effort canonicalize. If it fails (e.g., the path doesn't
        // exist on disk yet, which shouldn't happen at this stage), fall
        // back to the path as given — symlinking will then surface a
        // clearer error than mysterious missing files.
        let canonical = consumer_root
            .canonicalize_utf8()
            .unwrap_or_else(|_| consumer_root.to_owned());
        Self {
            archetect,
            consumer_id: hash_consumer_id(&canonical),
            consumer_root: canonical,
        }
    }

    /// Resolve and stage every catalog entry where `library == true`,
    /// including transitive library dependencies declared by staged libraries.
    ///
    /// Transitive deps are staged so that a library can `require()` its own
    /// sub-libraries without the consumer needing to declare them. Each
    /// library is fully encapsulated — the consumer only sees what it
    /// declares; libraries see their own declared dependencies.
    ///
    /// The staging dir for this consumer is **cleared and recreated** on
    /// every call. Existing staged entries from a previous render are
    /// removed before new entries are staged.
    pub fn stage(
        &mut self,
        catalog: &LinkedHashMap<String, CatalogEntry>,
    ) -> Result<Vec<StagedLibrary>, LibraryStagingError> {
        let staging_root = self.staging_root()?;

        // Wipe-and-recreate. The staging dir is purely derived state.
        if staging_root.exists() {
            fs::remove_dir_all(&staging_root).map_err(|e| {
                LibraryStagingError::StagingIo {
                    entry: "<staging-root>".to_string(),
                    detail: format!("could not clear staging dir {}: {}", staging_root, e),
                }
            })?;
        }

        let mut staged = Vec::new();
        // Track (name, normalized-source) pairs to prevent duplicate staging
        // of the same library under the same name. Using both components
        // allows diamond dependencies with different declared names to each
        // get their own staging entry while still avoiding true duplicates.
        let mut visited: HashSet<String> = HashSet::new();

        for (name, entry) in catalog {
            if !entry.library {
                continue;
            }
            let Some(source) = entry.source.as_deref() else {
                debug!(
                    "library entry `{}` has no source — skipping (catalog groups are not libraries)",
                    name
                );
                continue;
            };
            let source_abs = self.normalize_source(source);
            if let Err(err) = self.stage_transitive(name, &source_abs, &staging_root, &mut staged, &mut visited) {
                // A failed library is a hard error — the consumer
                // declared `library: true` and expects it to be
                // available. Bubble the error up so the script
                // doesn't try to require() something that's missing.
                warn!("library staging failed for `{}`: {}", name, err);
                return Err(err);
            }
        }

        Ok(staged)
    }

    /// Stage one library and recursively stage all of its own library
    /// dependencies before pushing itself onto `staged`. This ensures that
    /// when a library's init.lua is loaded, its sub-libraries are already on
    /// `package.path` and `require()` calls within the library succeed.
    ///
    /// `source` must already be normalized to an absolute path or git URL
    /// (relative-path normalization must happen at the call site, relative to
    /// the appropriate base — the consumer's root for top-level entries, the
    /// owning library's root for transitive deps).
    fn stage_transitive(
        &mut self,
        name: &str,
        source: &str,
        staging_root: &Utf8Path,
        staged: &mut Vec<StagedLibrary>,
        visited: &mut HashSet<String>,
    ) -> Result<(), LibraryStagingError> {
        // Dedup by (name, source) — prevents re-staging the exact same
        // entry and breaks circular dependency cycles.
        let key = format!("{}:{}", name, source);
        if !visited.insert(key) {
            return Ok(());
        }

        let library = self.stage_one(name, source, staging_root)?;

        // Recursively stage this library's own library dependencies.
        // Silently skip if the manifest can't be read — a library with no
        // readable manifest simply has no transitive deps.
        if let Ok(manifest) = Manifest::load(&library.source_root) {
            if let Some(dep_catalog) = manifest.catalog {
                for (dep_name, dep_entry) in &dep_catalog {
                    if !dep_entry.library {
                        continue;
                    }
                    let Some(dep_source) = dep_entry.source.as_deref() else {
                        continue;
                    };
                    // Normalize relative to the LIBRARY's source root, not
                    // the consumer's root. This is what makes encapsulation
                    // work: a library can declare deps with relative paths
                    // and they resolve relative to the library itself.
                    let dep_source_abs = normalize_source(&library.source_root, dep_source);
                    self.stage_transitive(dep_name, &dep_source_abs, staging_root, staged, visited)?;
                }
            }
        }

        staged.push(library);
        Ok(())
    }

    /// The synthetic staging root for this consumer. Lives under the
    /// archetect cache so it's transient and cleared by `archetect cache clear`.
    fn staging_root(&self) -> Result<Utf8PathBuf, LibraryStagingError> {
        let cache = self.archetect.layout().cache_dir();
        let root = cache.join("staging").join(&self.consumer_id);
        Ok(root)
    }

    /// Normalize a catalog source string. Git URLs and absolute paths are
    /// passed through unchanged. Relative local paths are interpreted as
    /// relative to the consumer's archetype root.
    ///
    /// This is intentionally conservative: it only rewrites paths that
    /// look like local filesystem paths AND aren't already absolute. URL
    /// detection mirrors what `SourceType::create` does — anything with
    /// `://` or matching the SSH-git pattern is left alone.
    fn normalize_source(&self, source: &str) -> String {
        normalize_source(&self.consumer_root, source)
    }

    /// Resolve and mount a single library. `source` must already be
    /// normalized (absolute path or git URL) — relative-path normalization
    /// is the caller's responsibility so that each library's deps resolve
    /// relative to the appropriate base.
    fn stage_one(
        &mut self,
        name: &str,
        source: &str,  // already normalized — no CWD-relative paths
        staging_root: &Utf8Path,
    ) -> Result<StagedLibrary, LibraryStagingError> {
        // Resolve the source via archetect's existing source layer
        // (handles git URLs, local paths, caching, etc.).
        let resolved = self.archetect.new_source(source).map_err(|err| {
            LibraryStagingError::SourceResolution {
                entry: name.to_string(),
                detail: err.to_string(),
            }
        })?;

        let resolved_path = resolved.path().map_err(|err| {
            LibraryStagingError::PathResolution {
                entry: name.to_string(),
                detail: err.to_string(),
            }
        })?;

        // 2. Identify the library's `lib/` and `includes/` directories on disk.
        //    Either or both may be absent — that's fine; staging skips them.
        let lib_source = resolved_path.join("lib");
        let includes_source = resolved_path.join("includes");

        // 3. Mount each present directory under <staging>/<kind>/<name>.
        //    This creates the namespace prefix that consumers see when they
        //    require/include with the library's local name.
        let lib_target_parent = staging_root.join("lib");
        let includes_target_parent = staging_root.join("includes");

        let lib_dir = if lib_source.exists() {
            ensure_dir(&lib_target_parent, name)?;
            let target = lib_target_parent.join(name);
            mount(&lib_source, &target, name)?;
            Some(target)
        } else {
            None
        };

        let includes_dir = if includes_source.exists() {
            ensure_dir(&includes_target_parent, name)?;
            let target = includes_target_parent.join(name);
            mount(&includes_source, &target, name)?;
            Some(target)
        } else {
            None
        };

        if lib_dir.is_none() && includes_dir.is_none() {
            warn!(
                "library entry `{}` has neither lib/ nor includes/ — nothing to stage",
                name
            );
        }

        Ok(StagedLibrary {
            name: name.to_string(),
            source: source.to_string(),
            lib_dir,
            includes_dir,
            source_root: resolved_path,
        })
    }
}

/// Normalize a catalog source string against a consumer archetype's root.
///
/// Git URLs and absolute paths are passed through unchanged. Relative
/// local paths are interpreted as relative to `consumer_root` and
/// rewritten to an absolute form when the joined path exists on disk. If
/// the joined path doesn't exist, the original source string is returned
/// unchanged so the downstream source resolver can produce its own
/// (better) error.
///
/// Shared by:
/// - `LibraryStager` (eager `library: true` resolution)
/// - The Lua `catalog.render` callsite (lazy resolution via `dispatch::dispatch`)
///
/// Both need the same normalization so authors can write portable
/// `source: "subdir"` declarations relative to their consumer archetype.
pub fn normalize_source(consumer_root: &Utf8Path, source: &str) -> String {
    if source.contains("://") || source.starts_with("git@") {
        return source.to_string();
    }
    let candidate = Utf8Path::new(source);
    if candidate.is_absolute() {
        return source.to_string();
    }
    let joined = consumer_root.join(candidate);
    if joined.exists() {
        joined.to_string()
    } else {
        source.to_string()
    }
}

/// Derive a stable identifier for a consumer archetype from its absolute
/// root path. Uses the standard library hasher — collision-resistant
/// enough for cache key purposes.
fn hash_consumer_id(consumer_root: &Utf8Path) -> String {
    let mut hasher = DefaultHasher::new();
    consumer_root.as_str().hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

/// Create the parent directory for a staging entry, returning a friendly
/// error if it can't be created.
fn ensure_dir(dir: &Utf8Path, entry_name: &str) -> Result<(), LibraryStagingError> {
    fs::create_dir_all(dir).map_err(|e| LibraryStagingError::StagingIo {
        entry: entry_name.to_string(),
        detail: format!("could not create staging parent {}: {}", dir, e),
    })
}

/// Mount `source` at `target` — symlink on Unix, copy on Windows.
///
/// The target's parent directory must already exist; this function only
/// performs the mount itself. If `target` already exists (e.g. from a stale
/// staging dir we missed clearing), it's removed first.
fn mount(source: &Utf8Path, target: &Utf8Path, entry_name: &str) -> Result<(), LibraryStagingError> {
    if target.exists() || target.is_symlink() {
        // Defensive cleanup — should not happen because the caller wipes
        // the staging dir, but a stale entry is harmless to remove.
        let _ = fs::remove_dir_all(target).or_else(|_| fs::remove_file(target));
    }

    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(source.as_std_path(), target.as_std_path()).map_err(|e| {
            LibraryStagingError::StagingIo {
                entry: entry_name.to_string(),
                detail: format!(
                    "could not symlink {} → {}: {}",
                    target, source, e
                ),
            }
        })
    }

    #[cfg(windows)]
    {
        copy_dir_recursive(source, target, entry_name)
    }
}

/// Recursive directory copy used as the Windows fallback when symlinks
/// aren't available without admin privileges.
#[cfg(windows)]
fn copy_dir_recursive(
    source: &Utf8Path,
    target: &Utf8Path,
    entry_name: &str,
) -> Result<(), LibraryStagingError> {
    fs::create_dir_all(target).map_err(|e| LibraryStagingError::StagingIo {
        entry: entry_name.to_string(),
        detail: format!("could not create {}: {}", target, e),
    })?;
    for entry in fs::read_dir(source).map_err(|e| LibraryStagingError::StagingIo {
        entry: entry_name.to_string(),
        detail: format!("could not read {}: {}", source, e),
    })? {
        let entry = entry.map_err(|e| LibraryStagingError::StagingIo {
            entry: entry_name.to_string(),
            detail: format!("dir entry error in {}: {}", source, e),
        })?;
        let path = entry.path();
        let utf8_path = Utf8PathBuf::from_path_buf(path.clone()).map_err(|_| {
            LibraryStagingError::StagingIo {
                entry: entry_name.to_string(),
                detail: format!("non-UTF8 path encountered while copying {}", source),
            }
        })?;
        let file_name = utf8_path.file_name().unwrap_or_default();
        let dest = target.join(file_name);
        if path.is_dir() {
            copy_dir_recursive(&utf8_path, &dest, entry_name)?;
        } else {
            fs::copy(&path, dest.as_std_path()).map_err(|e| {
                LibraryStagingError::StagingIo {
                    entry: entry_name.to_string(),
                    detail: format!("could not copy file {}: {}", utf8_path, e),
                }
            })?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::system::RootedSystemLayout;
    use std::fs;
    use tempfile::TempDir;

    fn build_archetect_and_layout() -> (TempDir, Archetect) {
        let temp = TempDir::new().unwrap();
        let layout = RootedSystemLayout::new(temp.path().to_str().unwrap()).unwrap();
        let archetect = Archetect::builder()
            .with_layout(layout)
            .build()
            .unwrap();
        (temp, archetect)
    }

    fn write_library_at(root: &Utf8Path, name: &str) -> Utf8PathBuf {
        let dir = root.join(name);
        let lib = dir.join("lib");
        let includes = dir.join("includes");
        fs::create_dir_all(&lib).unwrap();
        fs::create_dir_all(&includes).unwrap();
        fs::write(lib.join("hello.lua"), "return { greet = function() return \"hi\" end }").unwrap();
        fs::write(includes.join("header.atl"), "header content").unwrap();
        fs::write(
            dir.join("archetype.yaml"),
            "description: \"test library\"\nrequires:\n  archetect: \"3.0.0\"\n",
        )
        .unwrap();
        dir
    }

    fn make_catalog_entry(source: &str, library: bool) -> CatalogEntry {
        CatalogEntry {
            description: None,
            source: Some(source.to_string()),
            catalog: None,
            server: None,
            answers: None,
            switches: None,
            use_defaults: None,
            use_defaults_all: None,
            library,
            show: true,
        }
    }

    #[test]
    fn test_stage_skips_non_library_entries() {
        let (_temp, archetect) = build_archetect_and_layout();
        let workspace = TempDir::new().unwrap();
        let workspace_path = Utf8PathBuf::from_path_buf(workspace.path().to_path_buf()).unwrap();
        let lib_dir = write_library_at(&workspace_path, "inflect");

        let consumer_root = workspace_path.join("consumer");
        fs::create_dir_all(&consumer_root).unwrap();

        let mut catalog = LinkedHashMap::new();
        catalog.insert(
            "inflect".to_string(),
            make_catalog_entry(lib_dir.as_str(), false), // library: false
        );

        let mut stager = LibraryStager::new(archetect, &consumer_root);
        let staged = stager.stage(&catalog).unwrap();

        assert!(staged.is_empty(), "library: false entries are skipped");
    }

    #[test]
    fn test_stage_mounts_lib_and_includes() {
        let (_temp, archetect) = build_archetect_and_layout();
        let workspace = TempDir::new().unwrap();
        let workspace_path = Utf8PathBuf::from_path_buf(workspace.path().to_path_buf()).unwrap();
        let lib_source = write_library_at(&workspace_path, "inflect");

        let consumer_root = workspace_path.join("consumer");
        fs::create_dir_all(&consumer_root).unwrap();

        let mut catalog = LinkedHashMap::new();
        catalog.insert(
            "inflect-helpers".to_string(),
            make_catalog_entry(lib_source.as_str(), true),
        );

        let mut stager = LibraryStager::new(archetect, &consumer_root);
        let staged = stager.stage(&catalog).unwrap();

        assert_eq!(staged.len(), 1);
        let lib = &staged[0];
        assert_eq!(lib.name, "inflect-helpers");

        let lib_dir = lib.lib_dir.as_ref().expect("lib_dir should be set");
        let includes_dir = lib.includes_dir.as_ref().expect("includes_dir should be set");

        // Mounted under the consumer's chosen namespace, not the source's
        // physical name.
        assert!(lib_dir.ends_with("lib/inflect-helpers"));
        assert!(includes_dir.ends_with("includes/inflect-helpers"));

        // Files inside the staged dirs should be readable through the mount.
        assert!(lib_dir.join("hello.lua").exists());
        assert!(includes_dir.join("header.atl").exists());
    }

    #[test]
    fn test_stage_handles_library_with_only_lib() {
        let (_temp, archetect) = build_archetect_and_layout();
        let workspace = TempDir::new().unwrap();
        let workspace_path = Utf8PathBuf::from_path_buf(workspace.path().to_path_buf()).unwrap();
        let lib_dir = workspace_path.join("only-lib");
        fs::create_dir_all(lib_dir.join("lib")).unwrap();
        fs::write(lib_dir.join("lib").join("util.lua"), "return {}").unwrap();
        fs::write(
            lib_dir.join("archetype.yaml"),
            "description: \"only-lib\"\nrequires:\n  archetect: \"3.0.0\"\n",
        )
        .unwrap();

        let consumer_root = workspace_path.join("consumer");
        fs::create_dir_all(&consumer_root).unwrap();

        let mut catalog = LinkedHashMap::new();
        catalog.insert(
            "only-lib".to_string(),
            make_catalog_entry(lib_dir.as_str(), true),
        );

        let mut stager = LibraryStager::new(archetect, &consumer_root);
        let staged = stager.stage(&catalog).unwrap();

        assert_eq!(staged.len(), 1);
        assert!(staged[0].lib_dir.is_some());
        assert!(staged[0].includes_dir.is_none());
    }

    #[test]
    fn test_stage_recreates_on_each_call() {
        let (_temp, archetect) = build_archetect_and_layout();
        let workspace = TempDir::new().unwrap();
        let workspace_path = Utf8PathBuf::from_path_buf(workspace.path().to_path_buf()).unwrap();
        let lib_source = write_library_at(&workspace_path, "inflect");

        let consumer_root = workspace_path.join("consumer");
        fs::create_dir_all(&consumer_root).unwrap();

        let mut catalog = LinkedHashMap::new();
        catalog.insert(
            "inflect".to_string(),
            make_catalog_entry(lib_source.as_str(), true),
        );

        let mut stager = LibraryStager::new(archetect, &consumer_root);

        // First stage.
        let staged_1 = stager.stage(&catalog).unwrap();
        let lib_dir_1 = staged_1[0].lib_dir.clone().unwrap();
        assert!(lib_dir_1.exists());

        // Second stage — should clear and recreate without error.
        let staged_2 = stager.stage(&catalog).unwrap();
        let lib_dir_2 = staged_2[0].lib_dir.clone().unwrap();
        assert!(lib_dir_2.exists());
        // Same path because the consumer ID is stable for the same root.
        assert_eq!(lib_dir_1, lib_dir_2);
    }

    #[test]
    fn test_consumer_id_is_stable_for_same_root() {
        let path = Utf8PathBuf::from("/some/consumer/path");
        let id_a = hash_consumer_id(&path);
        let id_b = hash_consumer_id(&path);
        assert_eq!(id_a, id_b);
    }

    #[test]
    fn test_consumer_id_differs_for_different_roots() {
        let id_a = hash_consumer_id(Utf8Path::new("/consumer/a"));
        let id_b = hash_consumer_id(Utf8Path::new("/consumer/b"));
        assert_ne!(id_a, id_b);
    }

    // ── source_root and transitive staging ──────────────────────────────

    /// Create a library with a `lib/` dir and an optional set of catalog deps.
    /// `deps` is `(local_name, source_path)` — written verbatim into the catalog,
    /// so callers can pass absolute or relative paths as needed.
    fn write_library_with_catalog(root: &Utf8Path, name: &str, deps: &[(&str, &str)]) -> Utf8PathBuf {
        let dir = root.join(name);
        let lib = dir.join("lib");
        fs::create_dir_all(&lib).unwrap();
        fs::write(
            lib.join(format!("{}.lua", name.replace('-', "_"))),
            format!("return {{ name = '{}' }}", name),
        )
        .unwrap();

        let catalog_section = if deps.is_empty() {
            String::new()
        } else {
            let entries: String = deps
                .iter()
                .map(|(dep_name, dep_source)| {
                    format!(
                        "  {}:\n    source: \"{}\"\n    library: true\n",
                        dep_name, dep_source
                    )
                })
                .collect();
            format!("catalog:\n{}", entries)
        };

        fs::write(
            dir.join("archetype.yaml"),
            format!(
                "description: \"test library {}\"\nrequires:\n  archetect: \"3.0.0\"\n{}",
                name, catalog_section
            ),
        )
        .unwrap();
        dir
    }

    #[test]
    fn test_source_root_is_set_to_library_on_disk_path() {
        let (_temp, archetect) = build_archetect_and_layout();
        let workspace = TempDir::new().unwrap();
        let workspace_path = Utf8PathBuf::from_path_buf(workspace.path().to_path_buf()).unwrap();
        let lib_source = write_library_at(&workspace_path, "mylib");

        let consumer_root = workspace_path.join("consumer");
        fs::create_dir_all(&consumer_root).unwrap();

        let mut catalog = LinkedHashMap::new();
        catalog.insert("mylib".to_string(), make_catalog_entry(lib_source.as_str(), true));

        let mut stager = LibraryStager::new(archetect, &consumer_root);
        let staged = stager.stage(&catalog).unwrap();

        assert_eq!(staged.len(), 1);
        assert_eq!(staged[0].source_root, lib_source,
            "source_root should point to the library's actual on-disk root");
    }

    #[test]
    fn test_transitive_library_deps_are_staged() {
        let (_temp, archetect) = build_archetect_and_layout();
        let workspace = TempDir::new().unwrap();
        let workspace_path = Utf8PathBuf::from_path_buf(workspace.path().to_path_buf()).unwrap();

        // lib-c has no deps; lib-b declares lib-c as a library dep
        let lib_c = write_library_with_catalog(&workspace_path, "lib-c", &[]);
        let lib_b = write_library_with_catalog(&workspace_path, "lib-b", &[("lib-c", lib_c.as_str())]);

        let consumer_root = workspace_path.join("consumer");
        fs::create_dir_all(&consumer_root).unwrap();

        // Consumer only knows about lib-b
        let mut catalog = LinkedHashMap::new();
        catalog.insert("lib-b".to_string(), make_catalog_entry(lib_b.as_str(), true));

        let mut stager = LibraryStager::new(archetect, &consumer_root);
        let staged = stager.stage(&catalog).unwrap();

        // Both lib-b AND its transitive dep lib-c must be staged
        assert_eq!(staged.len(), 2,
            "transitive dep lib-c should be staged alongside lib-b; got {:?}",
            staged.iter().map(|l| &l.name).collect::<Vec<_>>());

        let names: Vec<&str> = staged.iter().map(|l| l.name.as_str()).collect();
        assert!(names.contains(&"lib-b"), "lib-b should be staged");
        assert!(names.contains(&"lib-c"), "lib-c (lib-b's dep) should be staged");

        // Both lib dirs must be accessible on disk
        let b_lib = staged.iter().find(|l| l.name == "lib-b").unwrap()
            .lib_dir.as_ref().expect("lib-b should have lib_dir");
        assert!(b_lib.exists(), "lib-b staging dir should exist on disk");

        let c_lib = staged.iter().find(|l| l.name == "lib-c").unwrap()
            .lib_dir.as_ref().expect("lib-c should have lib_dir");
        assert!(c_lib.exists(), "lib-c staging dir should exist on disk");
    }

    #[test]
    fn test_diamond_dep_staged_once_under_same_name() {
        let (_temp, archetect) = build_archetect_and_layout();
        let workspace = TempDir::new().unwrap();
        let workspace_path = Utf8PathBuf::from_path_buf(workspace.path().to_path_buf()).unwrap();

        // lib-c is a shared dep used by both lib-b and lib-d
        let lib_c = write_library_with_catalog(&workspace_path, "lib-c", &[]);
        let lib_b = write_library_with_catalog(&workspace_path, "lib-b", &[("lib-c", lib_c.as_str())]);
        let lib_d = write_library_with_catalog(&workspace_path, "lib-d", &[("lib-c", lib_c.as_str())]);

        let consumer_root = workspace_path.join("consumer");
        fs::create_dir_all(&consumer_root).unwrap();

        let mut catalog = LinkedHashMap::new();
        catalog.insert("lib-b".to_string(), make_catalog_entry(lib_b.as_str(), true));
        catalog.insert("lib-d".to_string(), make_catalog_entry(lib_d.as_str(), true));

        let mut stager = LibraryStager::new(archetect, &consumer_root);
        let staged = stager.stage(&catalog).unwrap();

        // lib-b, lib-d, and lib-c — lib-c should appear exactly once
        assert_eq!(staged.len(), 3,
            "lib-c should be staged once despite being a dep of both lib-b and lib-d; got {:?}",
            staged.iter().map(|l| &l.name).collect::<Vec<_>>());

        let c_count = staged.iter().filter(|l| l.name == "lib-c").count();
        assert_eq!(c_count, 1, "lib-c should appear exactly once in staged list");
    }

    #[test]
    fn test_circular_dependency_does_not_loop() {
        let (_temp, archetect) = build_archetect_and_layout();
        let workspace = TempDir::new().unwrap();
        let workspace_path = Utf8PathBuf::from_path_buf(workspace.path().to_path_buf()).unwrap();

        // lib-a and lib-b form a cycle. We must create the dirs first so the
        // paths exist before writing the archetype.yaml files that reference them.
        let lib_a_dir = workspace_path.join("lib-a");
        let lib_b_dir = workspace_path.join("lib-b");
        fs::create_dir_all(lib_a_dir.join("lib")).unwrap();
        fs::create_dir_all(lib_b_dir.join("lib")).unwrap();
        fs::write(lib_a_dir.join("lib").join("lib_a.lua"), "return {}").unwrap();
        fs::write(lib_b_dir.join("lib").join("lib_b.lua"), "return {}").unwrap();

        // lib-a → lib-b → lib-a (cycle)
        fs::write(
            lib_a_dir.join("archetype.yaml"),
            format!(
                "description: \"lib-a\"\nrequires:\n  archetect: \"3.0.0\"\ncatalog:\n  lib-b:\n    source: \"{}\"\n    library: true\n",
                lib_b_dir
            ),
        )
        .unwrap();
        fs::write(
            lib_b_dir.join("archetype.yaml"),
            format!(
                "description: \"lib-b\"\nrequires:\n  archetect: \"3.0.0\"\ncatalog:\n  lib-a:\n    source: \"{}\"\n    library: true\n",
                lib_a_dir
            ),
        )
        .unwrap();

        let consumer_root = workspace_path.join("consumer");
        fs::create_dir_all(&consumer_root).unwrap();

        let mut catalog = LinkedHashMap::new();
        catalog.insert("lib-a".to_string(), make_catalog_entry(lib_a_dir.as_str(), true));

        let mut stager = LibraryStager::new(archetect, &consumer_root);
        // Must complete without hanging or stack overflowing
        let result = stager.stage(&catalog);
        assert!(result.is_ok(), "circular deps should not cause an error or infinite loop");

        // lib-b should have been staged as lib-a's dep, but lib-a's back-edge
        // to lib-b should be skipped via the visited set
        let staged = result.unwrap();
        let names: Vec<&str> = staged.iter().map(|l| l.name.as_str()).collect();
        assert!(names.contains(&"lib-b"), "lib-b should be staged as lib-a's dep");
    }

    #[test]
    fn test_transitive_dep_relative_path_resolves_from_library_root() {
        let (_temp, archetect) = build_archetect_and_layout();
        let workspace = TempDir::new().unwrap();
        let workspace_path = Utf8PathBuf::from_path_buf(workspace.path().to_path_buf()).unwrap();

        // lib-c and lib-b sit at the same level under workspace/
        // lib-b references lib-c via a relative sibling path "../lib-c"
        write_library_with_catalog(&workspace_path, "lib-c", &[]);
        write_library_with_catalog(&workspace_path, "lib-b", &[("lib-c", "../lib-c")]);
        let lib_b = workspace_path.join("lib-b");

        // Consumer lives in a completely different directory. If relative paths
        // in lib-b's catalog were resolved against the consumer's root, "../lib-c"
        // would not find lib-c.
        let consumer_root = workspace_path.join("nested").join("deeply").join("consumer");
        fs::create_dir_all(&consumer_root).unwrap();
        fs::write(
            consumer_root.join("archetype.yaml"),
            "description: \"consumer\"\nrequires:\n  archetect: \"3.0.0\"\n",
        )
        .unwrap();

        let mut catalog = LinkedHashMap::new();
        catalog.insert("lib-b".to_string(), make_catalog_entry(lib_b.as_str(), true));

        let mut stager = LibraryStager::new(archetect, &consumer_root);
        let staged = stager.stage(&catalog).unwrap();

        let names: Vec<&str> = staged.iter().map(|l| l.name.as_str()).collect();
        assert!(
            names.contains(&"lib-c"),
            "lib-c should be staged because lib-b's '../lib-c' resolved correctly \
             relative to lib-b's source root, not the consumer's root; got: {:?}",
            names
        );
    }
}
