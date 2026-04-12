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
use std::fs;
use std::hash::{Hash, Hasher};

use camino::{Utf8Path, Utf8PathBuf};
use linked_hash_map::LinkedHashMap;
use log::{debug, warn};

use crate::manifest::CatalogEntry;
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

    /// Resolve and stage every catalog entry where `library == true`.
    /// Entries with `library == false` are skipped — they remain lazy and
    /// only get fetched if the script invokes `catalog.render(name)`.
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

            match self.stage_one(name, source, &staging_root) {
                Ok(library) => staged.push(library),
                Err(err) => {
                    // A failed library is a hard error — the consumer
                    // declared `library: true` and expects it to be
                    // available. Bubble the error up so the script
                    // doesn't try to require() something that's missing.
                    warn!("library staging failed for `{}`: {}", name, err);
                    return Err(err);
                }
            }
        }

        Ok(staged)
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

    fn stage_one(
        &mut self,
        name: &str,
        source: &str,
        staging_root: &Utf8Path,
    ) -> Result<StagedLibrary, LibraryStagingError> {
        // 0. If `source` is a relative local path (no git scheme, not
        //    absolute), interpret it as relative to the consumer's
        //    archetype root rather than the process CWD. Catalog authors
        //    expect their source declarations to be portable across the
        //    different directories archetect might be invoked from.
        let source_arg = self.normalize_source(source);

        // 1. Resolve the source via archetect's existing source layer
        //    (handles git URLs, local paths, caching, etc.).
        let resolved = self.archetect.new_source(&source_arg).map_err(|err| {
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
}
