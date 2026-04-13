use anyhow::Result;
use xshell::cmd;

use crate::util::sh;

pub fn run() -> Result<()> {
    let sh = sh::shell()?;
    println!("==> Duplicate dependencies");
    cmd!(sh, "cargo tree --workspace --duplicates").run()?;

    println!("\n==> Outdated dependencies");
    let outdated = cmd!(sh, "cargo outdated --workspace --root-deps-only")
        .quiet()
        .ignore_status()
        .run();
    if outdated.is_err() {
        eprintln!("cargo-outdated not installed; skipping.");
        eprintln!("install with: cargo install cargo-outdated");
    }
    Ok(())
}
