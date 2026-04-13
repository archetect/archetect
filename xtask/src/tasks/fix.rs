use anyhow::Result;
use xshell::cmd;

use crate::util::sh;

pub fn run() -> Result<()> {
    let sh = sh::shell()?;
    cmd!(sh, "cargo fmt --all").run()?;
    cmd!(sh, "cargo clippy --workspace --all-targets --all-features --fix --allow-dirty --allow-staged").run()?;
    Ok(())
}
