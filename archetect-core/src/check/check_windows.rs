use crate::errors::ArchetectError;
use super::CHECK_SUCCESS;
use super::CHECK_ERROR;
use super::CHECK_PREFIX;

pub fn perform_checks() -> Result<(), ArchetectError> {
    check_git_long_path_names()?;
    Ok(())
}

pub fn check_git_long_path_names() -> Result<(), ArchetectError> {
    println!("{CHECK_PREFIX} Checking Git Long Paths Support");
    if let Ok(config) = git2::Config::open_default() {
        let long_paths = config.get_bool("core.longpaths");

        match long_paths {
            Ok(enabled) => {
                if enabled {
                    println!("\t{CHECK_SUCCESS} Long Paths Enabled");
                } else {
                    print_git_long_paths_instructions();
                }
            }
            Err(_error) => {
                print_git_long_paths_instructions();
            }
        }
    }
    Ok(())
}

fn print_git_long_paths_instructions() {
    println!("\t{CHECK_ERROR} Long Paths Disabled");
    println!("\n\tArchetypes may have templates that contain long path names.");
    println!("\n\tExecute the following command to enable long path names within git on Windows:");
    println!("\tgit config --global core.longpaths true");
}

#[cfg(test)]
mod tests {
    use crate::check::check_windows::check_git_long_path_names;

    #[test]
    fn test_git_long_path_names() {
        check_git_long_path_names().expect("Working Code");
    }
}