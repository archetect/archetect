//! Low-level git operations with a try-git2-then-CLI-fallback strategy.
//!
//! The cache is content-addressed: a **bare mirror** per repo (`sources/<hash>/`) holds objects +
//! refs and is the fetch target; an immutable **working tree** per commit (`trees/<hash>/<oid>/`) is
//! materialized from it. Only fetch-class operations (clone, fetch, ls-remote) touch the network;
//! materialization is local. git2-first keeps the common public path working without the `git`
//! binary, falling back to `git` for auth (credential helpers, SSH agent, enterprise TLS).

use std::process::{Command, Stdio};

use camino::Utf8Path;
use log::debug;

use crate::error::GitCacheError;

/// Clone `url` into `dest` as a **bare mirror** (all branches + tags, no working tree). Tries `git2`
/// first; on any error, cleans up any partial state and falls back to `git clone --mirror`.
pub fn clone_mirror(url: &str, dest: &Utf8Path) -> Result<(), GitCacheError> {
    debug!("git-cache clone --mirror {} -> {}", url, dest);

    match clone_mirror_git2(url, dest) {
        Ok(()) => Ok(()),
        Err(err) => {
            debug!("git2 mirror clone failed ({err}); falling back to `git clone --mirror`");
            if dest.exists() {
                let _ = std::fs::remove_dir_all(dest.as_std_path());
            }
            let mut cmd = Command::new("git");
            cmd.args(["clone", "--mirror", "--quiet", url, dest.as_str()]);
            run_git(&mut cmd)
        }
    }
}

fn clone_mirror_git2(url: &str, dest: &Utf8Path) -> Result<(), git2::Error> {
    // A bare repo whose `origin` mirrors every ref, then an initial fetch to populate refs + tags.
    let repo = git2::build::RepoBuilder::new()
        .bare(true)
        .remote_create(|repo, name, url| repo.remote_with_fetch(name, url, "+refs/*:refs/*"))
        .clone(url, dest.as_std_path())?;
    // `RepoBuilder::clone` fetches the default branch; force a full mirror fetch so every branch and
    // tag is present locally (the content-addressed resolve needs them).
    fetch_all_refs(&repo)?;
    Ok(())
}

/// Fetch all refs (branches + tags) into the bare mirror at `mirror_dir`. git2-first, `git fetch`
/// fallback.
pub fn fetch_repo(mirror_dir: &Utf8Path) -> Result<(), GitCacheError> {
    debug!("git-cache fetch {}", mirror_dir);

    match git2::Repository::open_bare(mirror_dir.as_std_path()).and_then(|repo| fetch_all_refs(&repo)) {
        Ok(()) => Ok(()),
        Err(err) => {
            debug!("git2 fetch failed ({err}); falling back to `git fetch`");
            let mut cmd = Command::new("git");
            cmd.args([
                "--git-dir",
                mirror_dir.as_str(),
                "fetch",
                "--quiet",
                "--prune",
                "origin",
            ]);
            run_git(&mut cmd)
        }
    }
}

fn fetch_all_refs(repo: &git2::Repository) -> Result<(), git2::Error> {
    let mut remote = repo.find_remote("origin")?;
    let mut fo = git2::FetchOptions::new();
    fo.download_tags(git2::AutotagOption::All);
    fo.prune(git2::FetchPrune::On);
    // Force-update every ref; a mirror's configured refspec is `+refs/*:refs/*`.
    remote.fetch(&["+refs/*:refs/*"], Some(&mut fo), None)?;
    Ok(())
}

/// `git ls-remote <url> <gitref>` → the **peeled** commit OID hex the ref resolves to on the remote,
/// or `None` if the remote has no such ref. The cheap probe the hash gate uses. git2-first, CLI
/// fallback. `gitref` may be a short name (`v1`, `main`), a full ref, or `HEAD`.
pub fn ls_remote(url: &str, gitref: &str) -> Result<Option<String>, GitCacheError> {
    debug!("git-cache ls-remote {} {}", url, gitref);

    match ls_remote_via_git2(url, gitref) {
        Ok(found) => Ok(found),
        Err(err) => {
            debug!("git2 ls-remote failed ({err}); falling back to `git ls-remote`");
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

/// Resolve `gitref` against `(refname, oid)` advertisements, preferring the **peeled** (`^{}`) entry
/// for tags. Tags are tried before heads so a moving tag wins over a same-named branch.
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

/// Materialize the tree at commit `oid` (from the bare mirror at `mirror_dir`) into `tree_dir` — an
/// isolated, immutable working tree. The caller renders from it. git2 `checkout_tree` with a custom
/// `target_dir`; `git archive | tar` fallback. `tree_dir` must not already exist (caller uses a
/// temp-then-rename for atomicity).
pub fn materialize(mirror_dir: &Utf8Path, oid: &str, tree_dir: &Utf8Path) -> Result<(), GitCacheError> {
    debug!("git-cache materialize {} @ {} -> {}", mirror_dir, oid, tree_dir);

    match materialize_git2(mirror_dir, oid, tree_dir) {
        Ok(()) => Ok(()),
        Err(err) => {
            debug!("git2 materialize failed ({err}); falling back to `git archive`");
            materialize_via_cli(mirror_dir, oid, tree_dir)
        }
    }
}

fn materialize_git2(mirror_dir: &Utf8Path, oid: &str, tree_dir: &Utf8Path) -> Result<(), git2::Error> {
    let repo = git2::Repository::open_bare(mirror_dir.as_std_path())?;
    let oid = git2::Oid::from_str(oid)?;
    let tree = repo.find_commit(oid)?.tree()?;
    std::fs::create_dir_all(tree_dir.as_std_path())
        .map_err(|e| git2::Error::from_str(&format!("create tree dir: {e}")))?;
    let mut co = git2::build::CheckoutBuilder::new();
    co.target_dir(tree_dir.as_std_path())
        .update_index(false) // a bare mirror's index isn't ours to write
        .recreate_missing(true)
        .force();
    repo.checkout_tree(tree.as_object(), Some(&mut co))?;
    Ok(())
}

fn materialize_via_cli(mirror_dir: &Utf8Path, oid: &str, tree_dir: &Utf8Path) -> Result<(), GitCacheError> {
    std::fs::create_dir_all(tree_dir.as_std_path())?;
    // `git archive <oid>` writes a tar of the tree to stdout; extract it into tree_dir. No index or
    // HEAD mutation, so it's safe against a bare mirror.
    let mut archive = Command::new("git")
        .args(["--git-dir", mirror_dir.as_str(), "archive", "--format=tar", oid])
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|e| GitCacheError::Remote(format!("`git archive` could not run: {e}")))?;
    let stdout = archive
        .stdout
        .take()
        .ok_or_else(|| GitCacheError::Remote("git archive produced no output".to_string()))?;
    let tar_status = Command::new("tar")
        .args(["-x", "-C", tree_dir.as_str()])
        .stdin(stdout)
        .status()
        .map_err(|e| GitCacheError::Remote(format!("`tar` could not run: {e}")))?;
    let archive_status = archive
        .wait()
        .map_err(|e| GitCacheError::Remote(format!("git archive failed: {e}")))?;
    if !archive_status.success() {
        return Err(GitCacheError::Remote(format!(
            "git archive {oid} failed (exit {:?})",
            archive_status.code()
        )));
    }
    if !tar_status.success() {
        return Err(GitCacheError::Remote(format!(
            "tar extraction failed (exit {:?})",
            tar_status.code()
        )));
    }
    Ok(())
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
    fn missing_ref_is_none() {
        let entries = e(&[("refs/heads/main", "ccc")]);
        assert_eq!(resolve_ref_oid(&entries, "nope"), None);
    }
}
