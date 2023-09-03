use crate::v2::archetype::manifest::ArchetypeManifest;
use crate::v2::source::Source;
use crate::ArchetypeError;
use camino::{Utf8Path, Utf8PathBuf};
use std::fs;

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

    pub fn script_contents(&self) -> Result<String, ArchetypeError> {
        let mut script_path = self.root.clone();
        script_path.push(self.manifest().script());

        if !script_path.is_file() {
            return Err(ArchetypeError::ArchetypeManifestNotFound { path: script_path });
        }

        fs::read_to_string(script_path.as_std_path()).map_err(|err| ArchetypeError::IoError(err))
    }
}
