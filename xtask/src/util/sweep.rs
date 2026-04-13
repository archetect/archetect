use anyhow::{Context, Result};
use xshell::{cmd, Shell};

/// Ensure cargo-sweep is installed; install it quietly if missing.
pub fn ensure_installed(sh: &Shell) -> Result<()> {
    let status = cmd!(sh, "cargo sweep --version")
        .quiet()
        .ignore_stderr()
        .ignore_stdout()
        .run();
    if status.is_ok() {
        return Ok(());
    }
    eprintln!("cargo-sweep not found; installing...");
    cmd!(sh, "cargo install cargo-sweep --locked")
        .run()
        .context("failed to install cargo-sweep")?;
    Ok(())
}

/// Reap build artifacts older than `days` days.
pub fn reap(sh: &Shell, days: u32) -> Result<()> {
    ensure_installed(sh)?;
    let days_s = days.to_string();
    cmd!(sh, "cargo sweep --time {days_s}").run()?;
    Ok(())
}
