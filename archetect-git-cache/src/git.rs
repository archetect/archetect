//! Low-level git operations with a try-git2-then-CLI-fallback strategy.
//!
//! The cache is content-addressed: a **bare mirror** per repo (`sources/<hash>/`) holds objects +
//! refs and is the fetch target; an immutable **working tree** per commit (`trees/<hash>/<oid>/`) is
//! materialized from it. Only fetch-class operations (clone, fetch, ls-remote) touch the network;
//! materialization is local. git2-first keeps the common public path working without the `git`
//! binary, falling back to `git` for auth (credential helpers, SSH agent, enterprise TLS).

use std::process::{Command, Stdio};
use std::sync::Once;
use std::time::{Duration, Instant};

use camino::Utf8Path;
use log::debug;

use crate::error::GitCacheError;

/// The IO deadline for network git operations, in milliseconds. Overridable via
/// `ARCHETECT_GIT_TIMEOUT_MS` (tests use a short one; the value is latched by the first network
/// operation in the process).
fn network_timeout_ms() -> u64 {
    std::env::var("ARCHETECT_GIT_TIMEOUT_MS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(30_000)
}

/// Every network-touching operation gets a FINITE deadline. A stalled fetch used to block
/// forever — and `resolve` runs fetches inside the per-repo write lock, so one stalled fetch
/// froze every request queued behind that lock until the pod was restarted (archetect-server,
/// dev + prd, 2026-09-15). libgit2 gets global connect/IO timeouts here; the CLI fallbacks get
/// HTTP stall detection plus a wall-clock kill in [`run_git_bounded`].
fn ensure_network_timeouts() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| unsafe {
        let _ = git2::opts::set_server_connect_timeout_in_milliseconds(10_000);
        let _ = git2::opts::set_server_timeout_in_milliseconds(network_timeout_ms() as i32);
    });
}

/// Config args giving the `git` CLI the same stall discipline libgit2 gets from the opts above:
/// abort any HTTP transfer that stays under 1KB/s for the IO deadline.
fn cli_stall_args() -> [String; 4] {
    let secs = (network_timeout_ms() / 1000).max(1);
    [
        "-c".to_string(),
        "http.lowSpeedLimit=1000".to_string(),
        "-c".to_string(),
        format!("http.lowSpeedTime={secs}"),
    ]
}

/// Run a `git` command that may touch the network: never prompts, and is killed outright past the
/// wall clock (4x the IO deadline, floor 30s — a legitimately slow fetch gets minutes; a wedged
/// one gets killed instead of freezing the caller). Output is polled rather than streamed: every
/// invocation here produces small output (quiet clones/fetches, single-ref ls-remote), well under
/// the pipe buffer, so the child can never block on a full pipe while we poll.
fn run_git_bounded(command: &mut Command) -> Result<std::process::Output, GitCacheError> {
    command.env("GIT_TERMINAL_PROMPT", "0");
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let deadline = Duration::from_millis(network_timeout_ms().saturating_mul(4).max(30_000));
    let mut child = command
        .spawn()
        .map_err(|e| GitCacheError::Remote(format!("`git` CLI not available: {e}")))?;
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => {
                return child
                    .wait_with_output()
                    .map_err(|e| GitCacheError::Remote(format!("git: {e}")));
            }
            Ok(None) => {
                if start.elapsed() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(GitCacheError::Remote(format!(
                        "git timed out after {}s and was killed",
                        deadline.as_secs()
                    )));
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(e) => return Err(GitCacheError::Remote(format!("git wait failed: {e}"))),
        }
    }
}

/// Clone `url` into `dest` as a **bare mirror** (all branches + tags, no working tree). Tries `git2`
/// first; on any error, cleans up any partial state and falls back to `git clone --mirror`.
pub fn clone_mirror(url: &str, dest: &Utf8Path) -> Result<(), GitCacheError> {
    debug!("git-cache clone --mirror {} -> {}", url, dest);
    ensure_network_timeouts();

    match clone_mirror_git2(url, dest) {
        Ok(()) => Ok(()),
        Err(err) => {
            debug!("git2 mirror clone failed ({err}); falling back to `git clone --mirror`");
            if dest.exists() {
                let _ = std::fs::remove_dir_all(dest.as_std_path());
            }
            let mut cmd = Command::new("git");
            cmd.args(cli_stall_args());
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
    ensure_network_timeouts();

    match git2::Repository::open_bare(mirror_dir.as_std_path()).and_then(|repo| fetch_all_refs(&repo)) {
        Ok(()) => Ok(()),
        Err(err) => {
            debug!("git2 fetch failed ({err}); falling back to `git fetch`");
            let mut cmd = Command::new("git");
            cmd.args(cli_stall_args());
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
    ensure_network_timeouts();

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
    let mut cmd = Command::new("git");
    cmd.args(cli_stall_args());
    cmd.args(["ls-remote", url, gitref]);
    let output = run_git_bounded(&mut cmd)?;
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
    let output = run_git_bounded(command)?;
    match output.status.code() {
        Some(0) => Ok(()),
        Some(code) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(GitCacheError::Remote(format!("git exited {code}: {stderr}")))
        }
        None => Err(GitCacheError::Remote("git interrupted by signal".to_owned())),
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
