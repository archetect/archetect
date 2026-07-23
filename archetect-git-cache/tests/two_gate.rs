//! Freshness proofs for the content-addressed `resolve`: TTL gate, hash gate, force, offline,
//! immutable pins, missing refs. The TTL is controlled by the `interval` option (huge = fresh,
//! `ZERO` = expired); "no network" cases delete the remote so any network touch would error.

use std::path::Path;
use std::process::Command;
use std::time::Duration;

use archetect_git_cache::{resolve, FetchOptions, Freshness, GitCacheError, RefPin};
use camino::Utf8PathBuf;

fn git(args: &[&str], cwd: &Path) {
    let status = Command::new("git")
        .args(args)
        .current_dir(cwd)
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

fn scratch(tag: &str) -> Utf8PathBuf {
    let mut p = Utf8PathBuf::from_path_buf(std::env::temp_dir()).unwrap();
    p.push(format!("gitcache-fresh-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}

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

fn move_remote(remote: &Utf8PathBuf, content: &str) -> String {
    std::fs::write(remote.join("file.txt"), content).unwrap();
    git(&["add", "."], remote.as_std_path());
    git(&["commit", "-q", "-m", "second"], remote.as_std_path());
    git_out(&["rev-parse", "HEAD"], remote.as_std_path())
}

fn opts(force: bool, offline: bool, interval: Duration, pin: RefPin) -> FetchOptions {
    FetchOptions {
        force,
        offline,
        interval,
        pin,
    }
}

const FRESH: Duration = Duration::from_secs(3600);
const EXPIRED: Duration = Duration::ZERO;

fn let_ttl_expire() {
    std::thread::sleep(Duration::from_millis(20));
}

#[test]
fn absent_cache_clones_and_materializes() {
    let root = scratch("clone");
    let (remote, head) = init_remote(&root, "one");

    let r = resolve(remote.as_str(), None, &root, &opts(false, false, FRESH, RefPin::Infer)).unwrap();
    assert_eq!(r.freshness, Freshness::Cloned);
    assert_eq!(r.oid, head);
    assert_eq!(std::fs::read_to_string(r.tree_dir.join("file.txt")).unwrap(), "one");
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn within_ttl_uses_cache_with_zero_network() {
    let root = scratch("ttl-fresh");
    let (remote, _head) = init_remote(&root, "one");

    let first = resolve(remote.as_str(), None, &root, &opts(false, false, FRESH, RefPin::Infer)).unwrap();
    std::fs::remove_dir_all(&remote).unwrap(); // any network touch now would fail

    let r = resolve(remote.as_str(), None, &root, &opts(false, false, FRESH, RefPin::Infer)).unwrap();
    assert_eq!(r.freshness, Freshness::UpToDate { probed: false });
    assert_eq!(r.tree_dir, first.tree_dir); // same immutable tree
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn ttl_expired_but_remote_unchanged_stays_silent() {
    let root = scratch("ttl-match");
    let (remote, head) = init_remote(&root, "one");

    resolve(remote.as_str(), None, &root, &opts(false, false, FRESH, RefPin::Infer)).unwrap();
    let_ttl_expire();

    let r = resolve(
        remote.as_str(),
        None,
        &root,
        &opts(false, false, EXPIRED, RefPin::Infer),
    )
    .unwrap();
    assert_eq!(r.freshness, Freshness::UpToDate { probed: true });
    assert_eq!(r.oid, head);
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn ttl_expired_and_remote_moved_updates() {
    let root = scratch("ttl-differ");
    let (remote, first_oid) = init_remote(&root, "one");

    let r1 = resolve(remote.as_str(), None, &root, &opts(false, false, FRESH, RefPin::Infer)).unwrap();
    let second_oid = move_remote(&remote, "two");
    assert_ne!(first_oid, second_oid);
    let_ttl_expire();

    let r2 = resolve(
        remote.as_str(),
        None,
        &root,
        &opts(false, false, EXPIRED, RefPin::Infer),
    )
    .unwrap();
    assert_eq!(r2.freshness, Freshness::Updated);
    assert_eq!(r2.oid, second_oid);
    assert_ne!(r2.tree_dir, r1.tree_dir); // a new immutable tree, not a re-checkout
    assert_eq!(std::fs::read_to_string(r2.tree_dir.join("file.txt")).unwrap(), "two");
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn force_updates_regardless_of_ttl() {
    let root = scratch("force");
    let (remote, first) = init_remote(&root, "one");

    resolve(remote.as_str(), None, &root, &opts(false, false, FRESH, RefPin::Infer)).unwrap();
    let second = move_remote(&remote, "two");
    assert_ne!(first, second);

    let r = resolve(remote.as_str(), None, &root, &opts(true, false, FRESH, RefPin::Infer)).unwrap();
    assert_eq!(r.freshness, Freshness::Updated);
    assert_eq!(r.oid, second);
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn offline_uncached_errors_and_cached_uses_cache() {
    let root = scratch("offline");
    let (remote, _head) = init_remote(&root, "one");

    let err = resolve(remote.as_str(), None, &root, &opts(false, true, FRESH, RefPin::Infer)).unwrap_err();
    assert!(matches!(err, GitCacheError::OfflineAndNotCached(_)), "got {err:?}");

    resolve(remote.as_str(), None, &root, &opts(false, false, FRESH, RefPin::Infer)).unwrap();
    std::fs::remove_dir_all(&remote).unwrap();
    let r = resolve(remote.as_str(), None, &root, &opts(false, true, EXPIRED, RefPin::Infer)).unwrap();
    assert!(matches!(r.freshness, Freshness::UpToDate { .. }));
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn immutable_pin_never_probes_past_ttl() {
    let root = scratch("immutable");
    let (remote, _head) = init_remote(&root, "one");
    git(&["tag", "v1"], remote.as_std_path());

    resolve(
        remote.as_str(),
        Some("v1"),
        &root,
        &opts(false, false, FRESH, RefPin::Immutable),
    )
    .unwrap();
    std::fs::remove_dir_all(&remote).unwrap();
    let_ttl_expire();

    let r = resolve(
        remote.as_str(),
        Some("v1"),
        &root,
        &opts(false, false, EXPIRED, RefPin::Immutable),
    )
    .unwrap();
    assert!(matches!(r.freshness, Freshness::UpToDate { .. }));
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn missing_requested_ref_forces_fetch_within_ttl() {
    let root = scratch("missing-ref");
    let (remote, _head) = init_remote(&root, "one");

    resolve(remote.as_str(), None, &root, &opts(false, false, FRESH, RefPin::Infer)).unwrap();
    git(&["tag", "v9"], remote.as_std_path());
    let tag_oid = git_out(&["rev-parse", "v9^{commit}"], remote.as_std_path());

    let r = resolve(
        remote.as_str(),
        Some("v9"),
        &root,
        &opts(false, false, FRESH, RefPin::Infer),
    )
    .unwrap();
    assert_eq!(r.freshness, Freshness::Updated);
    assert_eq!(r.oid, tag_oid);
    std::fs::remove_dir_all(&root).ok();
}

/// Re-point tag `name` at a NEW commit — the floating-major move (`v1` tracking v1.x.y).
fn retag(remote: &Utf8PathBuf, name: &str, content: &str) -> String {
    let oid = move_remote(remote, content);
    git(&["tag", "-f", name], remote.as_std_path());
    oid
}

#[test]
fn inferred_tag_follows_a_move_past_ttl() {
    let root = scratch("tag-moves");
    let (remote, first) = init_remote(&root, "one");
    git(&["tag", "v1"], remote.as_std_path());

    let r1 = resolve(remote.as_str(), Some("v1"), &root, &opts(false, false, FRESH, RefPin::Infer)).unwrap();
    assert_eq!(r1.oid, first);

    let second = retag(&remote, "v1", "two");
    assert_ne!(first, second);
    let_ttl_expire();

    // A tag is a symbolic ref: past the TTL it takes the hash gate and follows the move.
    let r2 = resolve(remote.as_str(), Some("v1"), &root, &opts(false, false, EXPIRED, RefPin::Infer)).unwrap();
    assert_eq!(r2.freshness, Freshness::Updated);
    assert_eq!(r2.oid, second);
    assert_eq!(std::fs::read_to_string(r2.tree_dir.join("file.txt")).unwrap(), "two");
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn inferred_tag_unchanged_probes_silently() {
    let root = scratch("tag-still");
    let (remote, head) = init_remote(&root, "one");
    git(&["tag", "v1"], remote.as_std_path());

    resolve(remote.as_str(), Some("v1"), &root, &opts(false, false, FRESH, RefPin::Infer)).unwrap();
    let_ttl_expire();

    // Unmoved tag past the TTL: one probe, hash matches, no fetch — silent.
    let r = resolve(remote.as_str(), Some("v1"), &root, &opts(false, false, EXPIRED, RefPin::Infer)).unwrap();
    assert_eq!(r.freshness, Freshness::UpToDate { probed: true });
    assert_eq!(r.oid, head);
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn inferred_bare_rev_never_probes() {
    let root = scratch("rev-pin");
    let (remote, first) = init_remote(&root, "one");

    resolve(remote.as_str(), Some(&first), &root, &opts(false, false, FRESH, RefPin::Infer)).unwrap();
    move_remote(&remote, "two");
    let_ttl_expire();

    // A bare commit id is content-addressed: past the TTL it is not even probed.
    let r = resolve(remote.as_str(), Some(&first), &root, &opts(false, false, EXPIRED, RefPin::Infer)).unwrap();
    assert_eq!(r.freshness, Freshness::UpToDate { probed: false });
    assert_eq!(r.oid, first);
    std::fs::remove_dir_all(&root).ok();
}
