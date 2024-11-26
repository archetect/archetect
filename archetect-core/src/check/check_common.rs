use crate::errors::ArchetectError;
use std::process::Command;
use super::CHECK_SUCCESS;
use super::CHECK_ERROR;
use super::CHECK_PREFIX;

pub fn perform_checks() -> Result<(), ArchetectError> {
    check_git_installed()?;
    check_git_author()?;
    Ok(())
}

pub fn check_git_installed() -> Result<(), ArchetectError> {
    println!("\n{CHECK_PREFIX} Checking Git Installation");

    match  Command::new("git").arg("--version").output() {
        Ok(output) => {
            if output.status.success() {
                println!("\t{CHECK_SUCCESS} {}", String::from_utf8(output.stdout.trim_ascii().to_owned()).expect("UTF-8 Version String"));
            } else {
                println!("\t{CHECK_ERROR} Git was found, but returned an unexpected Status Code: {}",  output.status.code().unwrap());

                println!("\n\t Ensure that git is installed correctly.");
            }
        }
        Err(_error) => {
            println!("\tGit is required, but was not found on the PATH");
            println!("\n\t Ensure that git is installed correctly.");
        }
    }
    Ok(())
}

pub fn check_git_author() -> Result<(), ArchetectError> {
    println!("\n{CHECK_PREFIX} Checking Git User Name and Email");
    if let Ok(config) = git2::Config::open_default() {
        let name = config.get_string("user.name");
        let email = config.get_string("user.email");

        if let (Ok(name), Ok(email)) = (name, email)  {
            if !name.is_empty() && !email.is_empty() {
                println!("\t{CHECK_SUCCESS} {} <{}>", name, email);
            }
        } else {
            println!("\t{CHECK_ERROR} Git User Name or Email is empty.  Archetypes may use your Git\n\
            User Name and Email to answer questions about code authorship.");

            println!("\n\tExecute the following command to configure git:");
            println!("\n\tgit config --global user.name \"<your name>\"");
            println!("\tgit config --global user.email \"<your email>\"");
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