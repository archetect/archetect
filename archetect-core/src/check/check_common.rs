use std::env;
use std::fs;
use std::process::Command;

use crate::Archetect;
use crate::configuration::ShellExecPolicy;
use crate::errors::ArchetectError;

use super::{error, hint, header, info, pass, warn};

pub fn perform_checks(archetect: &Archetect) -> Result<(), ArchetectError> {
    check_git_installed()?;
    check_git_author()?;
    check_cache_dir(archetect)?;
    check_shell_exec_policy(archetect);
    check_lua_annotations(archetect);
    check_github_token();
    Ok(())
}

pub fn check_git_installed() -> Result<(), ArchetectError> {
    header("Git installation (optional)");

    match Command::new("git").arg("--version").output() {
        Ok(output) => {
            if output.status.success() {
                let version = String::from_utf8_lossy(output.stdout.trim_ascii()).to_string();
                pass(&version);
            } else {
                let code = output.status.code().unwrap_or(-1);
                warn(format!("Git found but returned status {}", code));
                hint("Ensure git is installed correctly if you plan to clone private repos or publish projects.");
            }
        }
        Err(_) => {
            // Archetect can clone public repos via libgit2 and open cached
            // repos directly. The `git` CLI is only needed for auth
            // (private repos, SSH) and for commit hooks / signing on the
            // rendered project's initial commit.
            warn("Git was not found on PATH");
            hint("Archetect works without `git` for public archetypes.");
            hint("Install git (https://git-scm.com/downloads) to clone private repos,");
            hint("run pre-commit hooks, or sign commits on rendered projects.");
        }
    }
    Ok(())
}

pub fn check_git_author() -> Result<(), ArchetectError> {
    header("Git user.name and user.email");

    let config = match git2::Config::open_default() {
        Ok(c) => c,
        Err(_) => {
            warn("Could not read git config");
            return Ok(());
        }
    };

    let name = config.get_string("user.name").ok();
    let email = config.get_string("user.email").ok();

    match (name.as_deref(), email.as_deref()) {
        (Some(n), Some(e)) if !n.is_empty() && !e.is_empty() => {
            pass(format!("{} <{}>", n, e));
        }
        _ => {
            warn("user.name or user.email is not set");
            hint("Archetypes use these as default answers for code authorship.");
            hint("Configure git with:");
            hint("  git config --global user.name \"<your name>\"");
            hint("  git config --global user.email \"<your email>\"");
        }
    }
    Ok(())
}

pub fn check_cache_dir(archetect: &Archetect) -> Result<(), ArchetectError> {
    header("Cache directory");

    let cache_dir = archetect.layout().cache_dir();

    if !cache_dir.exists() {
        // Try to create it — that's where archetect would put it on first run
        match fs::create_dir_all(&cache_dir) {
            Ok(_) => {
                pass(format!("{} (created)", cache_dir));
            }
            Err(e) => {
                error(format!("Cannot create {}: {}", cache_dir, e));
                hint("Archetect cannot pull or cache archetypes without a writable cache directory.");
                return Ok(());
            }
        }
    } else if !cache_dir.is_dir() {
        error(format!("{} exists but is not a directory", cache_dir));
        return Ok(());
    } else {
        // Test writability by attempting a probe file
        let probe = cache_dir.join(".archetect-probe");
        match fs::write(&probe, b"") {
            Ok(_) => {
                let _ = fs::remove_file(&probe);
                pass(format!("{} (writable)", cache_dir));
            }
            Err(e) => {
                error(format!("{} is not writable: {}", cache_dir, e));
                hint("Check directory permissions.");
            }
        }
    }
    Ok(())
}

pub fn check_shell_exec_policy(archetect: &Archetect) {
    header("Shell execution policy");

    let policy = archetect.configuration().shell_exec_policy();
    match policy {
        ShellExecPolicy::Forbidden => {
            info("Forbidden — archetype scripts cannot run shell commands");
        }
        ShellExecPolicy::Prompt => {
            info("Prompt — scripts must request approval per command (default)");
        }
        ShellExecPolicy::Allowed => {
            info("Allowed — scripts can run any shell command without prompting");
            hint("Set via --allow-exec, ARCHETECT_ALLOW_EXEC, or security.allow_exec in config.");
        }
    }
}

pub fn check_lua_annotations(archetect: &Archetect) {
    header("Lua IDE annotations");

    let annotations_dir = archetect.layout().data_dir().join("lua/annotations");
    let main_file = annotations_dir.join("archetect.lua");

    if main_file.is_file() {
        info(format!("Installed at {}", annotations_dir));
    } else {
        info("Not installed");
        hint("Run `archetect ide setup` to install Lua type annotations for IDE autocomplete.");
    }
}

pub fn check_github_token() {
    header("GITHUB_TOKEN environment variable");

    match env::var("GITHUB_TOKEN") {
        Ok(token) if !token.is_empty() => {
            // Don't print the token — just confirm it's set
            info(format!("Set ({} chars)", token.len()));
        }
        _ => {
            info("Not set");
            hint("Required only if archetype scripts use the archetect.github module.");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::system::RootedSystemLayout;

    #[test]
    fn test_check_git_installed() {
        // Just verify it doesn't panic — output is informational
        check_git_installed().expect("check should not error");
    }

    #[test]
    fn test_check_git_author() {
        check_git_author().expect("check should not error");
    }

    #[test]
    fn test_check_cache_dir_with_temp_layout() {
        let layout = RootedSystemLayout::temp().unwrap();
        let archetect = Archetect::builder()
            .with_layout(layout)
            .build()
            .unwrap();
        check_cache_dir(&archetect).expect("check should not error");
    }

    #[test]
    fn test_check_shell_exec_policy() {
        let layout = RootedSystemLayout::temp().unwrap();
        let archetect = Archetect::builder()
            .with_layout(layout)
            .build()
            .unwrap();
        // Just verify it doesn't panic
        check_shell_exec_policy(&archetect);
    }

    #[test]
    fn test_check_lua_annotations() {
        let layout = RootedSystemLayout::temp().unwrap();
        let archetect = Archetect::builder()
            .with_layout(layout)
            .build()
            .unwrap();
        check_lua_annotations(&archetect);
    }

    #[test]
    fn test_check_github_token() {
        check_github_token();
    }
}
