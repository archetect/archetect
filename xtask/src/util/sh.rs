use anyhow::Result;
use xshell::Shell;

use super::workspace;

/// Return an `xshell::Shell` rooted at the workspace root.
pub fn shell() -> Result<Shell> {
    let sh = Shell::new()?;
    sh.change_dir(workspace::root()?);
    Ok(sh)
}
