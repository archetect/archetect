use anyhow::{Context, Result};
use std::fs;
use xshell::cmd;

use crate::util::{sh, sweep, workspace};

/// Default targets baked in at generation time.
const DEFAULT_TARGETS: &[&str] = &[
    
];

pub fn run(override_targets: Vec<String>) -> Result<()> {
    let sh = sh::shell()?;
    let targets: Vec<String> = if !override_targets.is_empty() {
        override_targets
    } else if DEFAULT_TARGETS.is_empty() {
        vec![host_target(&sh)?]
    } else {
        DEFAULT_TARGETS.iter().map(|s| s.to_string()).collect()
    };

    let bins = workspace::bins()?;
    if bins.is_empty() {
        anyhow::bail!("no binary targets found to distribute");
    }

    let out_dir = workspace::root()?.join("target").join("dist");
    fs::create_dir_all(&out_dir)?;

    for target in &targets {
        println!("==> Building for {target}");
        cmd!(sh, "cargo build --release --workspace --target {target}").run()?;

        for bin in &bins {
            let bin_path = workspace::root()?
                .join("target")
                .join(target)
                .join("release")
                .join(bin_name(bin, target));
            if !bin_path.exists() {
                continue;
            }
            let archive = out_dir.join(format!("{bin}-{target}.tar.gz"));
            package(&sh, &bin_path, &archive, target)
                .with_context(|| format!("packaging {bin} for {target}"))?;
            println!("    packaged {}", archive.display());
        }
    }

    sweep::reap(&sh, 7)?;
    Ok(())
}

fn host_target(sh: &xshell::Shell) -> Result<String> {
    let output = cmd!(sh, "rustc -vV").read()?;
    for line in output.lines() {
        if let Some(rest) = line.strip_prefix("host: ") {
            return Ok(rest.trim().to_string());
        }
    }
    anyhow::bail!("could not determine host target from rustc output")
}

fn bin_name(bin: &str, target: &str) -> String {
    if target.contains("windows") {
        format!("{bin}.exe")
    } else {
        bin.to_string()
    }
}

fn package(
    sh: &xshell::Shell,
    bin_path: &std::path::Path,
    archive: &std::path::Path,
    target: &str,
) -> Result<()> {
    let parent = bin_path.parent().unwrap();
    let file = bin_path.file_name().unwrap().to_string_lossy().to_string();
    let archive = archive.to_string_lossy().to_string();
    let _ = target;
    let _cwd = sh.push_dir(parent);
    cmd!(sh, "tar -czf {archive} {file}").run()?;
    Ok(())
}
