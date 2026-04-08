use crate::Archetect;
use crate::errors::ArchetectError;

mod check_common;
#[cfg(target_os = "windows")]
mod check_windows;

const CHECK_PREFIX: &str = "🔍";
const CHECK_PASS: &str = "🟢";
const CHECK_WARN: &str = "🟡";
const CHECK_ERROR: &str = "🔴";
const CHECK_INFO: &str = "ℹ ";

/// Run all environment diagnostic checks. Aggregates failures
/// but does not error — checks are informational.
pub fn check_all(archetect: &Archetect) -> Result<(), ArchetectError> {
    check_common::perform_checks(archetect)?;
    #[cfg(target_os = "windows")]
    check_windows::perform_checks()?;
    Ok(())
}

/// Print a check header.
pub(crate) fn header(label: &str) {
    println!("\n{} {}", CHECK_PREFIX, label);
}

/// Print a passing check result.
pub(crate) fn pass(message: impl AsRef<str>) {
    println!("\t{} {}", CHECK_PASS, message.as_ref());
}

/// Print a warning check result. Optionally include remediation hints on subsequent lines.
pub(crate) fn warn(message: impl AsRef<str>) {
    println!("\t{} {}", CHECK_WARN, message.as_ref());
}

/// Print an error check result.
pub(crate) fn error(message: impl AsRef<str>) {
    println!("\t{} {}", CHECK_ERROR, message.as_ref());
}

/// Print an informational check result (neither pass nor fail).
pub(crate) fn info(message: impl AsRef<str>) {
    println!("\t{} {}", CHECK_INFO, message.as_ref());
}

/// Print remediation hint indented under a check result.
pub(crate) fn hint(message: impl AsRef<str>) {
    println!("\t   {}", message.as_ref());
}
