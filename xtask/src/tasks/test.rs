use anyhow::Result;

use super::{it, ut};

pub fn run() -> Result<()> {
    ut::run()?;
    it::run()?;
    Ok(())
}
