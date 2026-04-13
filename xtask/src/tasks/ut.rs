use anyhow::Result;
use xshell::cmd;

use crate::util::sh;

pub fn run() -> Result<()> {
    let sh = sh::shell()?;
    cmd!(sh, "cargo test --workspace --lib --bins").run()?;
    Ok(())
}
