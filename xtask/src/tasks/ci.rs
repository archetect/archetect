use anyhow::Result;

use super::{check, test};

pub fn run() -> Result<()> {
    check::run()?;
    test::run()?;
    Ok(())
}
