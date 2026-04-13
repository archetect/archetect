use anyhow::Result;

use crate::util::{sh, sweep};

pub fn run() -> Result<()> {
    let sh = sh::shell()?;
    sweep::reap(&sh, 7)?;
    Ok(())
}
