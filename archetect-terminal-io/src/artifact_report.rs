//! Reporting what a render produced.
//!
//! Goes to stdout, not the log: a caller driving archetect from a script or a
//! job wants to read the archive path or repo URL out of the output. Silence on
//! success is fine; silence when an artifact exists is a dead end for whoever
//! has to find the zip afterwards.

use archetect_api::{Artifact, ArtifactKind};

pub fn report_artifacts(artifacts: &[Artifact]) {
    if artifacts.is_empty() {
        return;
    }
    println!("Artifacts:");
    for artifact in artifacts {
        let kind = match artifact.kind {
            ArtifactKind::Archive => "archive",
            ArtifactKind::Repository => "repository",
        };
        println!("  {}: {}", kind, artifact.locator());
    }
}
