//! Low-level git operations with a try-git2-then-CLI-fallback strategy.
//!
//! Ported from archetect's `git_io.rs`. The goal is unchanged: **work without the `git` binary for
//! the common case** (a public clone/fetch/ls-remote over HTTPS), while falling back to the user's
//! installed `git` — with its credential helpers, SSH agent, and enterprise TLS — when auth is
//! required. Only fetch-class operations live here (clone, fetch, ls-remote): idempotent and
//! remote→local, so a fallback produces identical output state either way. Local-only reads use
//! `git2` directly in `lib.rs`.

use std::process::Command;

use camino::Utf8Path;
use log::debug;

use crate::error::GitCacheError;

/// Clone `url` into `dest`. Tries `git2` first; on any error, cleans up any partial checkout and
/// falls back to the `git` binary (which picks up credentials git2 doesn't).
pub fn clone(url: &str, dest: &Utf8Path) -> Result<(), GitCacheError> {
    debug!("git-cache clone {} -> {}", url, dest);

    match git2::Repository::clone(url, dest.as_std_path()) {
        Ok(_) => Ok(()),
        Err(err) => {
            debug!("git2 clone failed ({}); falling back to `git clone`", err);
            // `git clone` refuses a non-empty destination — remove any partial state git2 left.
            if dest.exists() {
                let _ = std::fs::remove_dir_all(dest.as_std_path());
            }
            let mut cmd = Command::new("git");
            cmd.args(["clone", url, dest.as_str(), "-q"]);
            run_git(&mut cmd)
        }
    }
}

/// Fetch all branches and tags for the repo at `repo_dir` (`--force --tags` semantics). Tries
/// `git2` first; on any error, falls back to `git fetch`.
pub fn fetch_repo(repo_dir: &Utf8Path) -> Result<(), GitCacheError> {
    debug!("git-cache fetch {}", repo_dir);

    match fetch_via_git2(repo_dir) {
        Ok(()) => Ok(()),
        Err(err) => {
            debug!("git2 fetch failed ({}); falling back to `git fetch`", err);
            let mut cmd = Command::new("git");
            cmd.current_dir(repo_dir).args(["fetch", "-q", "--force", "--tags"]);
            run_git(&mut cmd)
        }
    }
}

fn fetch_via_git2(repo_dir: &Utf8Path) -> Result<(), git2::Error> {
    let repo = git2::Repository::open(repo_dir.as_std_path())?;
    let mut remote = repo.find_remote("origin")?;

    let mut fo = git2::FetchOptions::new();
    fo.download_tags(git2::AutotagOption::All);

    // Force-update every configured refspec (prepend `+` if not already forced).
    let forced: Vec<String> = remote
        .fetch_refspecs()?
        .iter()
        .filter_map(|s| s.map(str::to_string))
        .map(|r| if r.starts_with('+') { r } else { format!("+{}", r) })
        .collect();

    remote.fetch(&forced, Some(&mut fo), None)?;
    Ok(())
}

/// `git ls-remote <url> <gitref>` → the **peeled** commit OID hex the ref resolves to on the
/// remote, or `None` if the remote has no such ref. This is the cheap probe the hash gate uses to
/// decide whether a full fetch is worth doing. git2-first, `git` CLI fallback.
///
/// `gitref` may be a short name (`v1`, `main`), a full ref (`refs/tags/v1`), or `HEAD` (the
/// default-branch probe). Tags are peeled (`^{}`) so the returned OID is the commit a moving/annotated
/// tag points at — matching what a checkout of that tag resolves to, so the baseline comparison is
/// apples-to-apples.
pub fn ls_remote(url: &str, gitref: &str) -> Result<Option<String>, GitCacheError> {
    debug!("git-cache ls-remote {} {}", url, gitref);

    match ls_remote_via_git2(url, gitref) {
        Ok(found) => Ok(found),
        Err(err) => {
            debug!("git2 ls-remote failed ({}); falling back to `git ls-remote`", err);
            ls_remote_via_cli(url, gitref)
        }
    }
}

fn ls_remote_via_git2(url: &str, gitref: &str) -> Result<Option<String>, git2::Error> {
    let mut remote = git2::Remote::create_detached(url)?;
    remote.connect(git2::Direction::Fetch)?;
    let entries: Vec<(String, String)> = remote
        .list()?
        .iter()
        .map(|head| (head.name().to_string(), head.oid().to_string()))
        .collect();
    remote.disconnect()?;
    Ok(resolve_ref_oid(&entries, gitref))
}

fn ls_remote_via_cli(url: &str, gitref: &str) -> Result<Option<String>, GitCacheError> {
    // `--tags --heads` alone wouldn't include HEAD; pass the ref explicitly so all three cases
    // (branch, tag, HEAD) are covered, and request peeled tags with the default output (git prints
    // both `<oid>\t<ref>` and `<oid>\t<ref>^{}` lines).
    let output = Command::new("git")
        .args(["ls-remote", url, gitref])
        .output()
        .map_err(|e| GitCacheError::Remote(format!("`git ls-remote` could not run: {e}")))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(GitCacheError::Remote(format!(
            "git ls-remote {url} {gitref} failed: {}",
            stderr.trim()
        )));
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let entries: Vec<(String, String)> = text
        .lines()
        .filter_map(|line| {
            let mut parts = line.split('\t');
            let oid = parts.next()?.trim().to_string();
            let name = parts.next()?.trim().to_string();
            (!oid.is_empty() && !name.is_empty()).then_some((name, oid))
        })
        .collect();
    Ok(resolve_ref_oid(&entries, gitref))
}

/// Resolve `gitref` against a set of `(refname, oid)` advertisements, preferring the **peeled**
/// (`^{}`) entry for tags. Candidate names are tried tags-first then heads (matching archetect's
/// "tag or branch" resolution order), so a moving tag wins over a same-named branch.
fn resolve_ref_oid(entries: &[(String, String)], gitref: &str) -> Option<String> {
    let candidates: Vec<String> = if gitref == "HEAD" {
        vec!["HEAD".to_string()]
    } else if gitref.starts_with("refs/") {
        vec![gitref.to_string()]
    } else {
        vec![
            format!("refs/tags/{gitref}"),
            format!("refs/heads/{gitref}"),
            gitref.to_string(),
        ]
    };
    for cand in &candidates {
        let peeled = format!("{cand}^{{}}");
        if let Some((_, oid)) = entries.iter().find(|(n, _)| n == &peeled) {
            return Some(oid.clone());
        }
        if let Some((_, oid)) = entries.iter().find(|(n, _)| n == cand) {
            return Some(oid.clone());
        }
    }
    None
}

fn run_git(command: &mut Command) -> Result<(), GitCacheError> {
    match command.output() {
        Ok(output) => match output.status.code() {
            Some(0) => Ok(()),
            Some(code) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(GitCacheError::Remote(format!("git exited {code}: {stderr}")))
            }
            None => Err(GitCacheError::Remote("git interrupted by signal".to_owned())),
        },
        Err(err) => Err(GitCacheError::Remote(format!("`git` CLI not available: {err}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::resolve_ref_oid;

    fn e(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
        pairs.iter().map(|(n, o)| (n.to_string(), o.to_string())).collect()
    }

    #[test]
    fn prefers_peeled_tag_oid() {
        let entries = e(&[("refs/tags/v1", "aaa"), ("refs/tags/v1^{}", "bbb")]);
        assert_eq!(resolve_ref_oid(&entries, "v1").as_deref(), Some("bbb"));
    }

    #[test]
    fn resolves_branch_by_short_name() {
        let entries = e(&[("refs/heads/main", "ccc"), ("HEAD", "ccc")]);
        assert_eq!(resolve_ref_oid(&entries, "main").as_deref(), Some("ccc"));
    }

    #[test]
    fn resolves_head() {
        let entries = e(&[("HEAD", "ddd"), ("refs/heads/main", "ddd")]);
        assert_eq!(resolve_ref_oid(&entries, "HEAD").as_deref(), Some("ddd"));
    }

    #[test]
    fn missing_ref_is_none() {
        let entries = e(&[("refs/heads/main", "ccc")]);
        assert_eq!(resolve_ref_oid(&entries, "nope"), None);
    }
}
