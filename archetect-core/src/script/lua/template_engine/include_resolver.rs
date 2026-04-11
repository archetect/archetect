//! Resolver for `{% include "path" %}` directives.
//!
//! Holds the configured includes directory and the stack of currently-
//! compiling includes (for cycle detection). Each include is read at
//! compile time and the contents are passed back to the compiler for
//! recursive inline tokenization.
//!
//! # Security
//!
//! Include paths are restricted to descendants of the includes directory:
//! absolute paths, `..` segments, and symlinks pointing outside the
//! includes tree are all rejected. The intent is that an archetype author
//! cannot use `{% include %}` as a side channel to read arbitrary files
//! from the host filesystem.

use std::fs;

use camino::{Utf8Path, Utf8PathBuf};

use super::error::TemplateCompileError;

/// Tracks the include directory and the active compile stack.
///
/// One resolver is constructed per top-level template render and is mutated
/// as the compiler descends into nested includes. The compiler pushes the
/// path onto the stack before descending and pops it on the way back up.
pub struct IncludeResolver {
    /// Ordered list of directories to search for includes. The first
    /// directory containing a matching file wins. Empty list = no resolution
    /// (any `{% include %}` errors out).
    ///
    /// Multiple dirs are needed because library archetypes contribute
    /// their own `includes/` directories that layer onto the consumer's
    /// own `includes/`. The consumer's dirs come first so consumer
    /// templates can shadow library partials with the same name.
    includes_dirs: Vec<Utf8PathBuf>,
    stack: Vec<Utf8PathBuf>,
}

impl IncludeResolver {
    /// Build a resolver from a list of include directories, searched in
    /// order. The first directory containing a matching file wins.
    ///
    /// If a directory doesn't exist on disk, it's silently skipped at
    /// resolution time — only an error if NO directory contains the
    /// requested include.
    pub fn new(includes_dirs: Vec<Utf8PathBuf>) -> Self {
        Self {
            includes_dirs,
            stack: Vec::new(),
        }
    }

    /// Convenience constructor for the single-directory case. Used in tests
    /// where the full multi-dir form would be needless ceremony.
    #[cfg(test)]
    pub fn single(includes_dir: Utf8PathBuf) -> Self {
        Self::new(vec![includes_dir])
    }

    /// A resolver with no includes directories configured. Any `{% include %}`
    /// directive will fail with `IncludeNotFound`. Used by callsites that
    /// don't have a manifest available (e.g. compiling a path-name template,
    /// rendering a one-off string).
    pub fn disabled() -> Self {
        Self {
            includes_dirs: Vec::new(),
            stack: Vec::new(),
        }
    }

    /// Read the included file at `relative` (as written in the template),
    /// pushing it onto the active compile stack.
    ///
    /// Returns the file contents and the canonical path that was added to
    /// the stack. The caller is responsible for calling `pop()` after the
    /// recursive compile completes.
    pub fn read(
        &mut self,
        relative: &str,
        line: usize,
    ) -> Result<(String, Utf8PathBuf), TemplateCompileError> {
        let resolved = self.resolve(relative, line)?;

        // Cycle detection — comparing resolved paths so symlink shenanigans
        // can't sneak past.
        if self.stack.iter().any(|p| p == &resolved) {
            let mut chain: Vec<String> = self
                .stack
                .iter()
                .map(|p| p.as_str().to_string())
                .collect();
            chain.push(resolved.as_str().to_string());
            return Err(TemplateCompileError::IncludeCycle {
                path: relative.to_string(),
                stack: chain,
            });
        }

        let contents = fs::read_to_string(&resolved).map_err(|err| {
            TemplateCompileError::IncludeReadError {
                path: relative.to_string(),
                line,
                detail: err.to_string(),
            }
        })?;

        self.stack.push(resolved.clone());
        Ok((contents, resolved))
    }

    /// Pop the most recently pushed include off the stack. The caller must
    /// ensure this matches the path returned by the corresponding `read`.
    pub fn pop(&mut self) {
        self.stack.pop();
    }

    /// Resolve `relative` against the configured includes directories.
    /// Searches each directory in order; the first one containing a
    /// matching file wins. Each candidate is sandbox-checked against its
    /// own root after canonicalization to prevent symlink escape.
    fn resolve(
        &self,
        relative: &str,
        line: usize,
    ) -> Result<Utf8PathBuf, TemplateCompileError> {
        if self.includes_dirs.is_empty() {
            return Err(TemplateCompileError::IncludeNotFound {
                path: relative.to_string(),
                line,
            });
        }

        // Reject absolute paths and any `..` segment up front. These checks
        // are independent of which directory we'd search and apply once.
        let candidate = Utf8Path::new(relative);
        if candidate.is_absolute() {
            return Err(TemplateCompileError::IncludeNotFound {
                path: relative.to_string(),
                line,
            });
        }
        if candidate.components().any(|c| c.as_str() == "..") {
            return Err(TemplateCompileError::IncludeNotFound {
                path: relative.to_string(),
                line,
            });
        }

        // Walk each include directory in order. First match wins.
        for includes_dir in &self.includes_dirs {
            let joined = includes_dir.join(candidate);
            if !joined.exists() {
                continue;
            }

            // Defense in depth — canonicalize and confirm the file is still
            // inside THIS directory after symlink resolution. If a symlink
            // would escape this dir, fall through to the next candidate.
            let Ok(canonical_file) = joined.canonicalize_utf8() else {
                continue;
            };
            let Ok(canonical_root) = includes_dir.canonicalize_utf8() else {
                continue;
            };

            if canonical_file.starts_with(&canonical_root) {
                return Ok(canonical_file);
            }
        }

        Err(TemplateCompileError::IncludeNotFound {
            path: relative.to_string(),
            line,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write(dir: &Utf8Path, name: &str, contents: &str) -> Utf8PathBuf {
        let path = dir.join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&path, contents).unwrap();
        path
    }

    fn temp_includes() -> (TempDir, Utf8PathBuf) {
        let tmp = TempDir::new().unwrap();
        let includes = Utf8PathBuf::from_path_buf(tmp.path().join("includes")).unwrap();
        fs::create_dir_all(&includes).unwrap();
        (tmp, includes)
    }

    #[test]
    fn test_read_basic() {
        let (_tmp, includes) = temp_includes();
        write(&includes, "header.atl", "Hello {{ name }}!");
        let mut resolver = IncludeResolver::single(includes);
        let (contents, _) = resolver.read("header.atl", 1).unwrap();
        assert_eq!(contents, "Hello {{ name }}!");
    }

    #[test]
    fn test_read_nested_directory() {
        let (_tmp, includes) = temp_includes();
        write(&includes, "partials/header.atl", "Header");
        let mut resolver = IncludeResolver::single(includes);
        let (contents, _) = resolver.read("partials/header.atl", 1).unwrap();
        assert_eq!(contents, "Header");
    }

    #[test]
    fn test_read_missing_returns_not_found() {
        let (_tmp, includes) = temp_includes();
        let mut resolver = IncludeResolver::single(includes);
        let err = resolver.read("missing.atl", 5).unwrap_err();
        assert!(
            matches!(
                err,
                TemplateCompileError::IncludeNotFound { line: 5, .. }
            ),
            "got {:?}",
            err
        );
    }

    #[test]
    fn test_disabled_resolver_always_not_found() {
        let mut resolver = IncludeResolver::disabled();
        let err = resolver.read("header.atl", 1).unwrap_err();
        assert!(matches!(err, TemplateCompileError::IncludeNotFound { .. }));
    }

    #[test]
    fn test_absolute_path_rejected() {
        let (_tmp, includes) = temp_includes();
        let mut resolver = IncludeResolver::single(includes);
        let err = resolver.read("/etc/passwd", 1).unwrap_err();
        assert!(matches!(err, TemplateCompileError::IncludeNotFound { .. }));
    }

    #[test]
    fn test_dotdot_rejected() {
        let (_tmp, includes) = temp_includes();
        let mut resolver = IncludeResolver::single(includes);
        let err = resolver.read("../escape.atl", 1).unwrap_err();
        assert!(matches!(err, TemplateCompileError::IncludeNotFound { .. }));
    }

    #[test]
    fn test_cycle_detection() {
        let (_tmp, includes) = temp_includes();
        write(&includes, "a.atl", "{% include \"b.atl\" %}");
        write(&includes, "b.atl", "{% include \"a.atl\" %}");
        let mut resolver = IncludeResolver::single(includes);

        // Push a then attempt to push it again — should fail.
        let _ = resolver.read("a.atl", 1).unwrap();
        let err = resolver.read("a.atl", 2).unwrap_err();
        assert!(
            matches!(err, TemplateCompileError::IncludeCycle { .. }),
            "got {:?}",
            err
        );
    }

    #[test]
    fn test_pop_reduces_stack() {
        let (_tmp, includes) = temp_includes();
        write(&includes, "a.atl", "x");
        let mut resolver = IncludeResolver::single(includes);
        resolver.read("a.atl", 1).unwrap();
        assert_eq!(resolver.stack.len(), 1);
        resolver.pop();
        assert_eq!(resolver.stack.len(), 0);
    }

    // ---------- Multi-directory search ----------

    #[test]
    fn test_multi_dir_first_match_wins() {
        // Two include dirs, both have header.atl. The first one in the
        // list wins (this is how a consumer's own includes/ shadows a
        // library include of the same name).
        let tmp = TempDir::new().unwrap();
        let dir_first = Utf8PathBuf::from_path_buf(tmp.path().join("first")).unwrap();
        let dir_second = Utf8PathBuf::from_path_buf(tmp.path().join("second")).unwrap();
        std::fs::create_dir_all(&dir_first).unwrap();
        std::fs::create_dir_all(&dir_second).unwrap();
        write(&dir_first, "header.atl", "FIRST");
        write(&dir_second, "header.atl", "SECOND");

        let mut resolver = IncludeResolver::new(vec![dir_first, dir_second]);
        let (contents, _) = resolver.read("header.atl", 1).unwrap();
        assert_eq!(contents, "FIRST");
    }

    #[test]
    fn test_multi_dir_falls_through_to_second() {
        // First dir doesn't have the file; second dir does.
        let tmp = TempDir::new().unwrap();
        let dir_first = Utf8PathBuf::from_path_buf(tmp.path().join("first")).unwrap();
        let dir_second = Utf8PathBuf::from_path_buf(tmp.path().join("second")).unwrap();
        std::fs::create_dir_all(&dir_first).unwrap();
        std::fs::create_dir_all(&dir_second).unwrap();
        write(&dir_second, "footer.atl", "footer content");

        let mut resolver = IncludeResolver::new(vec![dir_first, dir_second]);
        let (contents, _) = resolver.read("footer.atl", 1).unwrap();
        assert_eq!(contents, "footer content");
    }

    #[test]
    fn test_multi_dir_not_found_in_any() {
        let tmp = TempDir::new().unwrap();
        let dir_a = Utf8PathBuf::from_path_buf(tmp.path().join("a")).unwrap();
        let dir_b = Utf8PathBuf::from_path_buf(tmp.path().join("b")).unwrap();
        std::fs::create_dir_all(&dir_a).unwrap();
        std::fs::create_dir_all(&dir_b).unwrap();

        let mut resolver = IncludeResolver::new(vec![dir_a, dir_b]);
        let err = resolver.read("missing.atl", 7).unwrap_err();
        assert!(matches!(
            err,
            TemplateCompileError::IncludeNotFound { line: 7, .. }
        ));
    }

    #[test]
    fn test_multi_dir_namespace_prefix() {
        // The library staging convention: each library's includes/ is
        // mounted under a namespace dir. The resolver sees a single
        // staging root and the namespace is part of the include path.
        let tmp = TempDir::new().unwrap();
        let staging = Utf8PathBuf::from_path_buf(tmp.path().join("staging")).unwrap();
        let lib_includes = staging.join("inflect-helpers");
        std::fs::create_dir_all(&lib_includes).unwrap();
        write(&lib_includes, "header.atl", "INFLECT HEADER");

        let mut resolver = IncludeResolver::new(vec![staging]);
        let (contents, _) = resolver.read("inflect-helpers/header.atl", 1).unwrap();
        assert_eq!(contents, "INFLECT HEADER");
    }

    #[test]
    fn test_empty_dirs_list_acts_disabled() {
        let mut resolver = IncludeResolver::new(vec![]);
        let err = resolver.read("anything.atl", 1).unwrap_err();
        assert!(matches!(err, TemplateCompileError::IncludeNotFound { .. }));
    }
}
