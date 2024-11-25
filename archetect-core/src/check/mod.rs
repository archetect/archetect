use crate::errors::ArchetectError;

mod check_common;
#[cfg(target_os = "windows")]
mod check_windows;

const CHECK_PREFIX: &str = "🔍";
const CHECK_SUCCESS: &str = "🟢";
const CHECK_ERROR: &str = "🔴";

pub fn check_all() -> Result<(), ArchetectError> {
    check_common::perform_checks()?;
    #[cfg(target_os = "windows")]
    check_windows::perform_checks()?;
    Ok(())
}