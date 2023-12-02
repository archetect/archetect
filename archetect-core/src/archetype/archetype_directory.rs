use camino::{Utf8Path, Utf8PathBuf};

use crate::archetype::archetype_manifest::ArchetypeManifest;
use crate::errors::ArchetypeError;
use crate::source::Source;

#[derive(Clone, Debug)]
pub struct ArchetypeDirectory {
    manifest: ArchetypeManifest,
    root: Utf8PathBuf,
}

impl ArchetypeDirectory {
    pub fn new(source: Source) -> Result<ArchetypeDirectory, ArchetypeError> {
        let root = source.local_path().to_owned();
        let manifest = ArchetypeManifest::load(&root)?;

        Ok(ArchetypeDirectory { manifest, root })
    }

    pub fn manifest(&self) -> &ArchetypeManifest {
        &self.manifest
    }

    pub fn root(&self) -> &Utf8Path {
        self.root.as_ref()
    }

    pub fn modules_directory(&self) -> Utf8PathBuf {
        self.root.join(self.manifest().scripting().modules())
    }

    pub fn script(&self) -> Result<Utf8PathBuf, ArchetypeError> {
        let mut script_path = self.root.clone();
        script_path.push(self.manifest().scripting().main());

        if !script_path.is_file() {
            return Err(ArchetypeError::ArchetypeManifestNotFound { path: script_path });
        }

        Ok(script_path)
    }
}
