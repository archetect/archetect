use anyhow::Result;
use xshell::cmd;

use crate::util::{sh, sweep};

pub fn run(release: bool) -> Result<()> {
    let sh = sh::shell()?;
    let release_flag: &[&str] = if release { &["--release"] } else { &[] };
    cmd!(sh, "cargo build --workspace {release_flag...}").run()?;
    sweep::reap(&sh, 7)?;
    Ok(())
}
