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

    pub fn content_directory(&self) -> Utf8PathBuf {
        self.root().join(self.manifest().templating().content_directory())
    }

    pub fn template_directory(&self) -> Utf8PathBuf {
        self.root().join(self.manifest().templating().templates_directory())
    }

    pub fn render(&self, render_context: RenderContext) -> Result<ContextValue, ArchetypeError> {
        match self.directory().script() {
            Some(script_path) => {
                // Check for .rhai scripts and emit a helpful error
                if script_path.extension() == Some("rhai") {
                    let error_msg = format!(
                        "Rhai scripts (.rhai) are no longer supported in Archetect 3. \
                         This archetype uses '{}'. Please convert to Lua (.lua) or use \
                         Archetect 2.x to render this archetype.",
                        script_path
                    );
                    let _ = self.archetect.request(archetect_api::ScriptMessage::LogError(error_msg.clone()));
                    let _ = self.archetect.request(archetect_api::ScriptMessage::CompleteError(error_msg));
                    return Err(ArchetypeError::ScriptAbortError);
                }

                crate::script::lua::execute(self, &self.archetect, &render_context)
            }
            None => {
                // No script — if there are catalog entries, auto-present them
                if self.manifest().has_catalog() {
                    crate::catalog::auto_present_catalog(self, render_context)
                        .map_err(|e| match e {
                            ArchetectError::ArchetypeError(ae) => ae,
                            other => ArchetypeError::SourceError(
                                crate::errors::SourceError::SourceNotFound(other.to_string()),
                            ),
                        })
                } else {
                    let error_msg = format!(
                        "Archetype at '{}' has no script file and no catalog entries. \
                         Add an archetype.lua script or catalog entries to archetect.yaml.",
                        self.root()
                    );
                    let _ = self.archetect.request(archetect_api::ScriptMessage::LogError(error_msg.clone()));
                    Err(ArchetypeError::ScriptAbortError)
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
