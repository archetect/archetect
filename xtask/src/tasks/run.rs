use anyhow::{bail, Result};
use xshell::cmd;

use crate::util::{sh, workspace};

pub fn run(bin: Option<String>, release: bool, args: Vec<String>) -> Result<()> {
    let sh = sh::shell()?;
    let bins = workspace::bins()?;

    let bin = match bin {
        Some(b) => b,
        None => {
            if bins.is_empty() {
                bail!("no binary targets found in workspace");
            }
            println!("Available binaries:");
            for b in &bins {
                println!("  {b}");
            }
            return Ok(());
        }
    };

    if !bins.iter().any(|b| b == &bin) {
        bail!("binary '{bin}' not found. available: {}", bins.join(", "));
    }

    let release_flag: &[&str] = if release { &["--release"] } else { &[] };
    cmd!(sh, "cargo run -p {bin} {release_flag...} -- {args...}").run()?;
    Ok(())
}
