//! Git operations with a try-git2-then-CLI-fallback strategy.
//!
//! Archetect's goal: **work without the `git` binary for the common case**
//! (rendering a public archetype or catalog), while falling back to the
//! user's installed `git` — with all its credential helpers, SSH agent
//! integration, and enterprise TLS — when auth is required.
//!
//! ## Which operations live here
//!
//! Only **fetch-class** operations: clone, fetch, ls-remote. These are
//! idempotent and remote→local, so failure is observable and fallback
//! produces identical output state either way.
//!
//! ## Which operations do NOT live here
//!
//! - **Local-only reads** (status, log, config, tags, refs): use `git2`
//!   directly. No auth concerns, no fallback needed.
//! - **Local commits + push** (init, add, commit, push): use
//!   `Command::new("git")` directly. libgit2's `commit()` silently
//!   skips pre-commit / commit-msg hooks and GPG/SSH signing —
//!   "successful" is semantically wrong there, so fallback-on-failure
//!   never triggers. Keep those on the CLI.
//!
//! See `docs/plans/git2-transparent-fallback.md` for the design rationale.
use std::fs;
use std::process::Command;

use camino::Utf8Path;
use log::{debug, warn};

use crate::errors::SourceError;

/// Clone `url` into `dest`. Tries `git2` first; on any error, cleans up
/// any partial checkout and falls back to the `git` binary.
///
/// The fallback path is where authenticated clones succeed — `git` picks
/// up credential helpers, SSH agent keys, etc. that libgit2 doesn't.
pub fn clone(url: &str, dest: &Utf8Path) -> Result<(), SourceError> {
    debug!("git_io::clone {} -> {}", url, dest);

    match git2::Repository::clone(url, dest.as_std_path()) {
        Ok(_) => Ok(()),
        Err(err) => {
            debug!("git2 clone failed ({}); falling back to `git clone`", err);

            // Remove any partial state git2 may have created before the
            // CLI runs — `git clone` refuses a non-empty destination.
            if dest.exists() {
                let _ = fs::remove_dir_all(dest.as_std_path());
            }

            clone_via_cli(url, dest)
        }
    }
}

fn clone_via_cli(url: &str, dest: &Utf8Path) -> Result<(), SourceError> {
    let mut cmd = Command::new("git");
    cmd.args(["clone", url, dest.as_str(), "-q"]);
    run_git(&mut cmd)
}

/// Fetch all branches and tags for the repo at `repo_path`. Tries `git2`
/// first; on any error, falls back to `git fetch`.
///
/// Uses `--force --tags` semantics — matches the previous CLI invocation.
pub fn fetch(repo_path: &Utf8Path) -> Result<(), SourceError> {
    debug!("git_io::fetch {}", repo_path);

    match fetch_via_git2(repo_path) {
        Ok(()) => Ok(()),
        Err(err) => {
            debug!("git2 fetch failed ({}); falling back to `git fetch`", err);
            fetch_via_cli(repo_path)
        }
    }
}

fn fetch_via_git2(repo_path: &Utf8Path) -> Result<(), git2::Error> {
    let repo = git2::Repository::open(repo_path.as_std_path())?;
    let mut remote = repo.find_remote("origin")?;

    // Match the CLI's `--force --tags`: download all branches + all tags,
    // allowing non-fast-forward ref updates.
    let mut fo = git2::FetchOptions::new();
    fo.download_tags(git2::AutotagOption::All);

    let refspecs: Vec<String> = remote
        .fetch_refspecs()?
        .iter()
        .filter_map(|s| s.map(str::to_string))
        .collect();

    // Force-update: prepend `+` to each refspec if not already forced.
    let forced: Vec<String> = refspecs
        .into_iter()
        .map(|r| if r.starts_with('+') { r } else { format!("+{}", r) })
        .collect();

    remote.fetch(&forced, Some(&mut fo), None)?;
    Ok(())
}

fn fetch_via_cli(repo_path: &Utf8Path) -> Result<(), SourceError> {
    let mut cmd = Command::new("git");
    cmd.current_dir(repo_path)
        .args(["fetch", "-q", "--force", "--tags"]);
    run_git(&mut cmd)
}

fn run_git(command: &mut Command) -> Result<(), SourceError> {
    match command.output() {
        Ok(output) => match output.status.code() {
            Some(0) => Ok(()),
            Some(code) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(SourceError::RemoteSourceError(format!(
                    "git exited {}: {}",
                    code, stderr
                )))
            }
            None => Err(SourceError::RemoteSourceError(
                "git interrupted by signal".to_owned(),
            )),
        },
        Err(err) => {
            // `git` not installed. Surface a guidance-shaped error — this
            // is exactly the case we wanted to avoid ever hitting for the
            // common public-clone path, but when we DO hit it (auth
            // required), we want the user to know what to install.
            warn!("`git` CLI not available: {}", err);
            Err(SourceError::IoError(err))
        }
    }
}
