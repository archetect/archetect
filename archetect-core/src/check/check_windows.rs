use std::path::Path;
use winreg::enums::HKEY_LOCAL_MACHINE;
use winreg::RegKey;
use crate::errors::ArchetectError;
use super::CHECK_SUCCESS;
use super::CHECK_ERROR;
use super::CHECK_PREFIX;

pub fn perform_checks() -> Result<(), ArchetectError> {
    check_git_long_path_names()?;
    check_registry_long_path_names()?;
    Ok(())
}

pub fn check_git_long_path_names() -> Result<(), ArchetectError> {
    println!("\n{CHECK_PREFIX} Checking Git Long Paths Support");
    if let Ok(config) = git2::Config::open_default() {
        let long_paths = config.get_bool("core.longpaths");

        match long_paths {
            Ok(enabled) => {
                if enabled {
                    println!("\t{CHECK_SUCCESS} Git Long Paths Enabled");
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

pub fn check_registry_long_path_names() -> Result<(), ArchetectError> {
    println!("\n{CHECK_PREFIX} Checking Windows Long Paths Support");
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let fs_path = Path::new("SYSTEM").join("CurrentControlSet").join("Control").join("FileSystem");
    let fs = hklm.open_subkey(fs_path)?;
    let long_paths_enabled: u32 = fs.get_value("LongPathsEnabled")?;
    if long_paths_enabled != 0 {
        println!("\t{CHECK_SUCCESS} Windows Long Paths Enabled");
    } else {
        print_registry_long_path_names_instructions()?;
    }
    Ok(())
}

fn print_registry_long_path_names_instructions() -> Result<(), ArchetectError> {
    println!("\t{CHECK_ERROR} Windows Long Paths Disabled");
    println!("\n\tArchetypes may have templates that contain long path names.");
    println!("\n\tChange the value of HKEY_LOCAL_MACHINE\\SYSTEM\\CurrentControlSet\\Control\\FileSystem\\LongPathsEnabled\
    \n\tfrom '0' to '1' in the Windows Registry and restart your system.");

    Ok(())
}

fn print_git_long_paths_instructions() {
    println!("\t{CHECK_ERROR} Git Long Paths Disabled");
    println!("\n\tArchetypes may have templates that contain long path names.");
    println!("\n\tExecute the following command to enable long path names within git on Windows:");
    println!("\n\tgit config --global core.longpaths true");
}

#[cfg(test)]
mod tests {
    use crate::check::check_windows::check_git_long_path_names;

    #[test]
    fn test_git_long_path_names() {
        check_git_long_path_names().expect("Working Code");
    }
}