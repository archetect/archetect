use anyhow::{Context, Result};
use cargo_metadata::{Metadata, MetadataCommand};
use std::path::PathBuf;

pub fn metadata() -> Result<Metadata> {
    MetadataCommand::new()
        .no_deps()
        .exec()
        .context("failed to load cargo metadata")
}

pub fn root() -> Result<PathBuf> {
    Ok(metadata()?.workspace_root.clone().into_std_path_buf())
}

pub fn bins() -> Result<Vec<String>> {
    let md = metadata()?;
    let mut names = Vec::new();
    for pkg in md.workspace_packages() {
        if pkg.name == "xtask" {
            continue;
        }
        for tgt in &pkg.targets {
            if tgt.kind.iter().any(|k| k == "bin") {
                names.push(pkg.name.clone());
                break;
            }
        }
    }
    names.sort();
    names.dedup();
    Ok(names)
}
