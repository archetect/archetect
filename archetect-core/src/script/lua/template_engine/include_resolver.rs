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
    includes_dir: Option<Utf8PathBuf>,
    stack: Vec<Utf8PathBuf>,
}

impl IncludeResolver {
    /// Build a resolver pointing at `includes_dir`.
    ///
    /// If the directory doesn't exist on disk, that's not an error here —
    /// it only becomes one if a template actually uses `{% include %}`.
    pub fn new(includes_dir: Utf8PathBuf) -> Self {
        Self {
            includes_dir: Some(includes_dir),
            stack: Vec::new(),
        }
    }

    /// A resolver with no includes directory configured. Any `{% include %}`
    /// directive will fail with `IncludeNotFound`. Used by callsites that
    /// don't have a manifest available (e.g. compiling a path-name template,
    /// rendering a one-off string).
    pub fn disabled() -> Self {
        Self {
            includes_dir: None,
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

    /// Resolve `relative` against the configured includes directory and
    /// validate that the result is a descendant of it.
    fn resolve(
        &self,
        relative: &str,
        line: usize,
    ) -> Result<Utf8PathBuf, TemplateCompileError> {
        let Some(ref includes_dir) = self.includes_dir else {
            return Err(TemplateCompileError::IncludeNotFound {
                path: relative.to_string(),
                line,
            });
        };

        // Reject absolute paths and any `..` segment up front.
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

        let joined = includes_dir.join(candidate);
        if !joined.exists() {
            return Err(TemplateCompileError::IncludeNotFound {
                path: relative.to_string(),
                line,
            });
        }

        // Defense in depth — canonicalize and confirm the file is still
        // inside the includes directory after symlink resolution.
        let canonical_file = joined
            .canonicalize_utf8()
            .map_err(|err| TemplateCompileError::IncludeReadError {
                path: relative.to_string(),
                line,
                detail: err.to_string(),
            })?;
        let canonical_root = includes_dir.canonicalize_utf8().map_err(|err| {
            TemplateCompileError::IncludeReadError {
                path: relative.to_string(),
                line,
                detail: err.to_string(),
            }
        })?;

        if !canonical_file.starts_with(&canonical_root) {
            return Err(TemplateCompileError::IncludeNotFound {
                path: relative.to_string(),
                line,
            });
        }

        Ok(canonical_file)
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
        let mut resolver = IncludeResolver::new(includes);
        let (contents, _) = resolver.read("header.atl", 1).unwrap();
        assert_eq!(contents, "Hello {{ name }}!");
    }

    #[test]
    fn test_read_nested_directory() {
        let (_tmp, includes) = temp_includes();
        write(&includes, "partials/header.atl", "Header");
        let mut resolver = IncludeResolver::new(includes);
        let (contents, _) = resolver.read("partials/header.atl", 1).unwrap();
        assert_eq!(contents, "Header");
    }

    #[test]
    fn test_read_missing_returns_not_found() {
        let (_tmp, includes) = temp_includes();
        let mut resolver = IncludeResolver::new(includes);
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
        let mut resolver = IncludeResolver::new(includes);
        let err = resolver.read("/etc/passwd", 1).unwrap_err();
        assert!(matches!(err, TemplateCompileError::IncludeNotFound { .. }));
    }

    #[test]
    fn test_dotdot_rejected() {
        let (_tmp, includes) = temp_includes();
        let mut resolver = IncludeResolver::new(includes);
        let err = resolver.read("../escape.atl", 1).unwrap_err();
        assert!(matches!(err, TemplateCompileError::IncludeNotFound { .. }));
    }

    #[test]
    fn test_cycle_detection() {
        let (_tmp, includes) = temp_includes();
        write(&includes, "a.atl", "{% include \"b.atl\" %}");
        write(&includes, "b.atl", "{% include \"a.atl\" %}");
        let mut resolver = IncludeResolver::new(includes);

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
        let mut resolver = IncludeResolver::new(includes);
        resolver.read("a.atl", 1).unwrap();
        assert_eq!(resolver.stack.len(), 1);
        resolver.pop();
        assert_eq!(resolver.stack.len(), 0);
    }
}
