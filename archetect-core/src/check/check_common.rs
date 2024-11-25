use crate::errors::ArchetectError;
use std::process::Command;
use super::CHECK_SUCCESS;
use super::CHECK_ERROR;
use super::CHECK_PREFIX;

pub fn perform_checks() -> Result<(), ArchetectError> {
    check_git_installed()?;
    Ok(())
}

pub fn check_git_installed() -> Result<(), ArchetectError> {
    println!("\n{CHECK_PREFIX} Checking Git Installation");

    match  Command::new("git").arg("--version").output() {
        Ok(output) => {
            if output.status.success() {
                println!("\t{CHECK_SUCCESS} {}", String::from_utf8(output.stdout).expect("UTF-8 Version String"));
            } else {
                println!("\t{CHECK_ERROR} Git was found, but returned an unexpected Status Code: {}",  output.status.code().unwrap());

                println!("\n\t Ensure that git is installed correctly.");
            }
        }
        Err(error) => {
            println!("\t Git is required, but was not found on the PATH");
            println!("\n\t Ensure that git is installed correctly.");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::check::check_common::{perform_checks, check_git_installed};

    #[test]
    fn test_check_git() {
        perform_checks().expect("Working code");
    }

    #[test]
    fn test_git_version() {
        check_git_installed().expect("Working Code");
    }


}