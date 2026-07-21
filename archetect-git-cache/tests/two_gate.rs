//! Executable proofs of the two-gate freshness check, driven against real local git remotes.
//!
//! The TTL gate is controlled deterministically by the `interval` option — a huge interval keeps the
//! cache "fresh" (TTL never expires), `Duration::ZERO` forces expiry. The "no network happened"
//! cases are proved by **deleting the remote** after the initial clone: any code path that reaches
//! the network then errors, so a successful `UpToDate` is proof the network was never touched.

use std::path::Path;
use std::process::Command;
use std::time::Duration;

use archetect_git_cache::{fetch, FetchOptions, Freshness, GitCacheError, RefPin};
use camino::Utf8PathBuf;

// ── git test harness ────────────────────────────────────────────────────────────────────────────

fn git(args: &[&str], cwd: &Path) {
    let status = Command::new("git")
        .args(args)
        .current_dir(cwd)
        // Deterministic identity so `commit`/`tag` work on a bare CI runner.
        .env("GIT_AUTHOR_NAME", "gitcache")
        .env("GIT_AUTHOR_EMAIL", "gitcache@example.com")
        .env("GIT_COMMITTER_NAME", "gitcache")
        .env("GIT_COMMITTER_EMAIL", "gitcache@example.com")
        .status()
        .expect("run git");
    assert!(status.success(), "git {args:?} failed");
}

fn git_out(args: &[&str], cwd: &Path) -> String {
    let out = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("run git");
    assert!(out.status.success(), "git {args:?} failed");
    String::from_utf8(out.stdout).unwrap().trim().to_string()
}

/// A unique scratch root for a test (no rand/Date in tests — pid + a per-test tag).
fn scratch(tag: &str) -> Utf8PathBuf {
    let mut p = Utf8PathBuf::from_path_buf(std::env::temp_dir()).unwrap();
    p.push(format!("gitcache-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}

/// Build a non-bare remote with one commit on the default branch; return (remote_dir, head_oid).
fn init_remote(root: &Utf8PathBuf, first: &str) -> (Utf8PathBuf, String) {
    let remote = root.join("remote");
    std::fs::create_dir_all(&remote).unwrap();
    std::fs::write(remote.join("file.txt"), first).unwrap();
    git(&["init", "-q", "-b", "main"], remote.as_std_path());
    git(&["add", "."], remote.as_std_path());
    git(&["commit", "-q", "-m", "first"], remote.as_std_path());
    let oid = git_out(&["rev-parse", "HEAD"], remote.as_std_path());
    (remote, oid)
}

/// Add a commit to the remote's default branch; return the new head oid.
fn move_remote(remote: &Utf8PathBuf, content: &str) -> String {
    std::fs::write(remote.join("file.txt"), content).unwrap();
    git(&["add", "."], remote.as_std_path());
    git(&["commit", "-q", "-m", "second"], remote.as_std_path());
    git_out(&["rev-parse", "HEAD"], remote.as_std_path())
}

/// The OID recorded as the hash-gate baseline in the cache repo's config (there is one per test).
fn stored_oid(cache: &Utf8PathBuf) -> Option<String> {
    let out = Command::new("git")
        .args(["config", "--get-regexp", r"^gitcache\..*\.oid"])
        .current_dir(cache.as_std_path())
        .output()
        .ok()?;
    let text = String::from_utf8(out.stdout).ok()?;
    text.lines()
        .next()
        .and_then(|l| l.split_whitespace().nth(1))
        .map(str::to_string)
}

fn opts(force: bool, offline: bool, interval: Duration, pin: RefPin) -> FetchOptions {
    FetchOptions {
        force,
        offline,
        interval,
        pin,
    }
}

const FRESH: Duration = Duration::from_secs(3600); // TTL never expires within a test
const EXPIRED: Duration = Duration::ZERO; // TTL is always expired

/// Sleep just long enough that `now - checkedAt > 0`, so a `ZERO` interval reads as expired.
fn let_ttl_expire() {
    std::thread::sleep(Duration::from_millis(20));
}

// ── proofs ──────────────────────────────────────────────────────────────────────────────────────

#[test]
fn absent_cache_clones_and_records_baseline() {
    let root = scratch("clone");
    let (remote, head) = init_remote(&root, "one");
    let cache = root.join("cache");

    let out = fetch(remote.as_str(), None, &cache, &opts(false, false, FRESH, RefPin::Infer)).unwrap();

    assert_eq!(out.freshness, Freshness::Cloned);
    assert_eq!(out.resolved_oid, head);
    assert_eq!(stored_oid(&cache).as_deref(), Some(head.as_str()));
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn within_ttl_uses_cache_with_zero_network() {
    let root = scratch("ttl-fresh");
    let (remote, _head) = init_remote(&root, "one");
    let cache = root.join("cache");

    // Seed the cache, then DELETE the remote — a network touch now would fail.
    fetch(remote.as_str(), None, &cache, &opts(false, false, FRESH, RefPin::Infer)).unwrap();
    std::fs::remove_dir_all(&remote).unwrap();

    let out = fetch(remote.as_str(), None, &cache, &opts(false, false, FRESH, RefPin::Infer)).unwrap();
    assert_eq!(out.freshness, Freshness::UpToDate { probed: false });
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn ttl_expired_but_remote_unchanged_stays_silent() {
    let root = scratch("ttl-match");
    let (remote, head) = init_remote(&root, "one");
    let cache = root.join("cache");

    fetch(remote.as_str(), None, &cache, &opts(false, false, FRESH, RefPin::Infer)).unwrap();
    let_ttl_expire();

    // TTL expired → hash gate runs ls-remote → matches → no fetch, but a real network probe ran.
    let out = fetch(
        remote.as_str(),
        None,
        &cache,
        &opts(false, false, EXPIRED, RefPin::Infer),
    )
    .unwrap();
    assert_eq!(out.freshness, Freshness::UpToDate { probed: true });
    assert_eq!(out.resolved_oid, head); // unchanged
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn ttl_expired_and_remote_moved_updates() {
    let root = scratch("ttl-differ");
    let (remote, first) = init_remote(&root, "one");
    let cache = root.join("cache");

    fetch(remote.as_str(), None, &cache, &opts(false, false, FRESH, RefPin::Infer)).unwrap();
    let second = move_remote(&remote, "two");
    assert_ne!(first, second);
    let_ttl_expire();

    let out = fetch(
        remote.as_str(),
        None,
        &cache,
        &opts(false, false, EXPIRED, RefPin::Infer),
    )
    .unwrap();
    assert_eq!(out.freshness, Freshness::Updated);
    assert_eq!(out.resolved_oid, second);
    assert_eq!(stored_oid(&cache).as_deref(), Some(second.as_str())); // baseline advanced
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn force_updates_regardless_of_ttl() {
    let root = scratch("force");
    let (remote, first) = init_remote(&root, "one");
    let cache = root.join("cache");

    fetch(remote.as_str(), None, &cache, &opts(false, false, FRESH, RefPin::Infer)).unwrap();
    let second = move_remote(&remote, "two");
    assert_ne!(first, second);

    // Fresh TTL would normally short-circuit; force skips the gate entirely.
    let out = fetch(remote.as_str(), None, &cache, &opts(true, false, FRESH, RefPin::Infer)).unwrap();
    assert_eq!(out.freshness, Freshness::Updated);
    assert_eq!(out.resolved_oid, second);
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn offline_uncached_errors_and_cached_uses_cache() {
    let root = scratch("offline");
    let (remote, _head) = init_remote(&root, "one");
    let cache = root.join("cache");

    // Uncached + offline → a clear error, no network attempt.
    let err = fetch(remote.as_str(), None, &cache, &opts(false, true, FRESH, RefPin::Infer)).unwrap_err();
    assert!(matches!(err, GitCacheError::OfflineAndNotCached(_)), "got {err:?}");

    // Seed, delete remote, then offline → cache is used.
    fetch(remote.as_str(), None, &cache, &opts(false, false, FRESH, RefPin::Infer)).unwrap();
    std::fs::remove_dir_all(&remote).unwrap();
    let out = fetch(
        remote.as_str(),
        None,
        &cache,
        &opts(false, true, EXPIRED, RefPin::Infer),
    )
    .unwrap();
    assert!(matches!(out.freshness, Freshness::UpToDate { .. }));
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn immutable_pin_never_probes_past_ttl() {
    let root = scratch("immutable");
    let (remote, _head) = init_remote(&root, "one");
    git(&["tag", "v1"], remote.as_std_path());
    let cache = root.join("cache");

    // Cache the tag, then DELETE the remote: an immutable pin must not ls-remote or fetch.
    fetch(
        remote.as_str(),
        Some("v1"),
        &cache,
        &opts(false, false, FRESH, RefPin::Immutable),
    )
    .unwrap();
    std::fs::remove_dir_all(&remote).unwrap();
    let_ttl_expire();

    let out = fetch(
        remote.as_str(),
        Some("v1"),
        &cache,
        &opts(false, false, EXPIRED, RefPin::Immutable),
    )
    .unwrap();
    assert!(matches!(out.freshness, Freshness::UpToDate { .. }));
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn missing_requested_ref_forces_fetch_within_ttl() {
    let root = scratch("missing-ref");
    let (remote, _head) = init_remote(&root, "one");
    let cache = root.join("cache");

    // Cache the default branch (no tags yet).
    fetch(remote.as_str(), None, &cache, &opts(false, false, FRESH, RefPin::Infer)).unwrap();
    // Now the remote grows a tag the cache has never seen.
    git(&["tag", "v9"], remote.as_std_path());
    let tag_oid = git_out(&["rev-parse", "v9^{commit}"], remote.as_std_path());

    // Even within a fresh TTL, a ref missing from the cache forces a fetch to obtain it.
    let out = fetch(
        remote.as_str(),
        Some("v9"),
        &cache,
        &opts(false, false, FRESH, RefPin::Infer),
    )
    .unwrap();
    assert_eq!(out.freshness, Freshness::Updated);
    assert_eq!(out.resolved_oid, tag_oid);
    std::fs::remove_dir_all(&root).ok();
}
