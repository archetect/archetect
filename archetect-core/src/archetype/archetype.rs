use std::sync::Arc;

use camino::{Utf8Path, Utf8PathBuf};

use archetect_api::{ContextValue, ExistingFilePolicy};

use crate::Archetect;
use crate::archetype::archetype_directory::ArchetypeDirectory;
use crate::archetype::archetype_manifest::ArchetypeManifest;
use crate::archetype::render_context::RenderContext;
use crate::errors::{ArchetectError, ArchetypeError};
use crate::source::Source;

#[derive(Clone)]
pub struct Archetype {
    archetect: Archetect,
    pub(crate) inner: Arc<Inner>,
}

pub(crate) struct Inner {
    source: Option<Source>,
    pub directory: ArchetypeDirectory,
}

impl Archetype {
    pub fn new(archetect: Archetect, source: Source) -> Result<Archetype, ArchetypeError> {
        let directory = ArchetypeDirectory::new(source.path()?)?;
        let inner = Arc::new(Inner { directory, source: Some(source) });
        let archetype = Archetype { archetect, inner };

        Ok(archetype)
    }

    pub fn archetect(&self) -> &Archetect {
        &self.archetect
    }

    pub fn source(&self) -> &Option<Source> {
        &self.inner.source
    }

    pub fn directory(&self) -> &ArchetypeDirectory {
        &self.inner.directory
    }

    pub fn manifest(&self) -> &ArchetypeManifest {
        self.inner.directory.manifest()
    }

    pub fn root(&self) -> &Utf8Path {
        self.inner.directory.root()
    }

    /// Resolve a path relative to the archetype root.
    ///
    /// Phase 1 of catalog-driven dependencies removed the
    /// `templating.content` field — there is no separate "content
    /// directory" prefix. The script's `directory.render(path, context)`
    /// is interpreted directly as a root-relative path. This helper
    /// remains as a convenience for the wiring code.
    pub fn root_path<P: AsRef<Utf8Path>>(&self, relative: P) -> Utf8PathBuf {
        self.root().join(relative)
    }

    pub fn render(&self, render_context: RenderContext) -> Result<ContextValue, ArchetypeError> {
        self.render_with_action(render_context, None)
    }

    /// Render with an optional catalog action path. When `action` is `Some`,
    /// the path is resolved against the archetype's catalog entries —
    /// groups recurse, leaves render. When `None`, the catalog is presented
    /// as an interactive menu (or the script runs, if one exists).
    ///
    /// Called from the CLI `render` subcommand when the user passes an
    /// `[action]` positional argument (e.g., `archetect render <url> common/gitignore`).
    pub fn render_with_action(
        &self,
        render_context: RenderContext,
        action: Option<&str>,
    ) -> Result<ContextValue, ArchetypeError> {
        match self.directory().script() {
            Some(script_path) => {
                // Check for .rhai scripts and emit a helpful error
                if script_path.extension() == Some("rhai") {
                    let error_msg = format!(
                        "Rhai scripts (.rhai) are not supported. This archetype uses \
                         '{}'. Please convert to Lua (.lua), or use `archetect2` to \
                         render legacy archetypes.",
                        script_path
                    );
                    let _ = self.archetect.request(archetect_api::ScriptMessage::LogError(error_msg.clone()));
                    let _ = self.archetect.request(archetect_api::ScriptMessage::CompleteError(error_msg));
                    return Err(ArchetypeError::ScriptAbortError);
                }

                crate::script::lua::execute(self, &self.archetect, &render_context)
            }
            None => {
                // No script — if there are catalog entries, dispatch them.
                // If there are NO catalog entries either, this is a "library
                // archetype" — a repo that exists purely to expose lib/ and
                // includes/ for other archetypes to consume. Print a friendly
                // message and exit cleanly so the user understands the
                // situation isn't an error.
                if self.manifest().has_catalog() {
                    let catalog = self.manifest().catalog().ok_or_else(|| {
                        ArchetypeError::SourceError(
                            crate::errors::SourceError::SourceNotFound(
                                "No catalog entries found in manifest".to_string(),
                            ),
                        )
                    })?;
                    crate::catalog::dispatch::dispatch(
                        self.archetect(),
                        catalog,
                        action,
                        render_context,
                    )
                    .map_err(|e| match e {
                        ArchetectError::ArchetypeError(ae) => ae,
                        other => ArchetypeError::SourceError(
                            crate::errors::SourceError::SourceNotFound(other.to_string()),
                        ),
                    })
                } else {
                    let msg = format!(
                        "{} has no script and no catalog —\n\
                         it's probably a library, intended for use as a dependency.\n\
                         \n\
                         To use it from another archetype, declare it in your catalog:\n\
                         \n\
                         catalog:\n  \
                           <local-name>:\n    \
                             source: {}\n    \
                             library: true",
                        self.root().file_name().unwrap_or("(this archetype)"),
                        self.root(),
                    );
                    let _ = self.archetect.request(archetect_api::ScriptMessage::LogInfo(msg));
                    Ok(archetect_api::ContextValue::Nil)
                }
            }
        }
    }

    pub fn check_requirements(&self) -> Result<(), ArchetypeError> {
        self.manifest().requires().check_requirements(&self.archetect)?;
        Ok(())
    }
}

#[derive(Copy, Clone, Debug)]
pub enum OverwritePolicy {
    Overwrite,
    Preserve,
    Prompt,
}

impl Default for OverwritePolicy {
    fn default() -> Self {
        OverwritePolicy::Preserve
    }
}

impl From<OverwritePolicy> for ExistingFilePolicy {
    fn from(value: OverwritePolicy) -> Self {
        match value {
            OverwritePolicy::Overwrite => ExistingFilePolicy::Overwrite,
            OverwritePolicy::Preserve => ExistingFilePolicy::Preserve,
            OverwritePolicy::Prompt => ExistingFilePolicy::Prompt,
        }
    }
}

#[cfg(test)]
mod tests {}
