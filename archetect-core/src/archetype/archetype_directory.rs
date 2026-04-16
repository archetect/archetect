use camino::{Utf8Path, Utf8PathBuf};

use crate::archetype::archetype_manifest::ArchetypeManifest;
use crate::errors::ArchetypeError;

#[derive(Clone, Debug)]
pub struct ArchetypeDirectory {
    manifest: ArchetypeManifest,
    root: Utf8PathBuf,
}

impl ArchetypeDirectory {
    pub fn new(root: Utf8PathBuf) -> Result<ArchetypeDirectory, ArchetypeError> {
        let manifest = ArchetypeManifest::load(&root)?;
        Ok(ArchetypeDirectory { manifest, root })
    }

    pub fn manifest(&self) -> &ArchetypeManifest {
        &self.manifest
    }

    pub fn root(&self) -> &Utf8Path {
        self.root.as_ref()
    }

    /// The archetype's standardized Lua modules directory: `<root>/lib/`.
    ///
    /// Phase 1 of catalog-driven dependencies removed the `scripting.modules`
    /// manifest field. The author's own Lua helpers always live in `lib/`
    /// (which is automatically on `package.path` per commit 3) — no
    /// configuration needed.
    pub fn modules_directory(&self) -> Utf8PathBuf {
        self.root.join("lib")
    }

    /// Returns the script path if a script file exists, or `None` if this is a
    /// script-less archetype (e.g. a pure catalog).
    ///
    /// The entry-point filename is fixed — `archetype.lua` at the archetype
    /// root. (Legacy `archetype.rhai` is still detected so v2 archetypes
    /// produce the targeted error from the Lua runtime, which is clearer
    /// than "no script found".)
    pub fn script(&self) -> Option<Utf8PathBuf> {
        for name in &["archetype.lua", "archetype.rhai"] {
            let path = self.root.join(name);
            if path.is_file() {
                return Some(path);
            }
        }
        None
    }

    /// True if this archetype has a script file.
    pub fn has_script(&self) -> bool {
        self.script().is_some()
    }
}
