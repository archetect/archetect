//! The Phase-2 proofs: content-addressing (a tree per commit), long-lived sessions (an in-flight
//! session is never disturbed by, and never blocks, another moving the branch), and the reaper.

use std::path::Path;
use std::process::Command;
use std::time::Duration;

use archetect_git_cache::{prune, resolve, FetchOptions, RefPin};
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

fn scratch(tag: &str) -> Utf8PathBuf {
    let mut p = Utf8PathBuf::from_path_buf(std::env::temp_dir()).unwrap();
    p.push(format!("gitcache-ca-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}

fn init_remote(root: &Utf8PathBuf, first: &str) -> Utf8PathBuf {
    let remote = root.join("remote");
    std::fs::create_dir_all(&remote).unwrap();
    std::fs::write(remote.join("file.txt"), first).unwrap();
    git(&["init", "-q", "-b", "main"], remote.as_std_path());
    git(&["add", "."], remote.as_std_path());
    git(&["commit", "-q", "-m", "first"], remote.as_std_path());
    remote
}

fn move_remote(remote: &Utf8PathBuf, content: &str) {
    std::fs::write(remote.join("file.txt"), content).unwrap();
    git(&["add", "."], remote.as_std_path());
    git(&["commit", "-q", "-m", "moved"], remote.as_std_path());
}

fn opts(interval: Duration) -> FetchOptions {
    FetchOptions {
        force: false,
        offline: false,
        interval,
        pin: RefPin::Infer,
    }
}

const FRESH: Duration = Duration::from_secs(3600);
const EXPIRED: Duration = Duration::ZERO;

/// A moved branch materializes a *new* tree; the old commit's tree is left intact.
#[test]
fn each_commit_gets_its_own_immutable_tree() {
    let root = scratch("addr");
    let remote = init_remote(&root, "one");

    let a = resolve(remote.as_str(), Some("main"), &root, &opts(FRESH)).unwrap();
    assert_eq!(std::fs::read_to_string(a.tree_dir.join("file.txt")).unwrap(), "one");

    move_remote(&remote, "two");
    let b = resolve(remote.as_str(), Some("main"), &root, &opts(EXPIRED)).unwrap();

    assert_ne!(a.oid, b.oid);
    assert_ne!(a.tree_dir, b.tree_dir);
    // Both trees coexist; the old one is byte-for-byte untouched.
    assert_eq!(std::fs::read_to_string(a.tree_dir.join("file.txt")).unwrap(), "one");
    assert_eq!(std::fs::read_to_string(b.tree_dir.join("file.txt")).unwrap(), "two");
    std::fs::remove_dir_all(&root).ok();
}

/// An in-flight session (holding its lease "at lunch") is neither disturbed by nor blocks a
/// concurrent session that moves the branch — and two sessions on the same commit share the tree.
#[test]
fn long_lived_session_is_isolated_and_non_blocking() {
    let root = scratch("session");
    let remote = init_remote(&root, "one");

    // Session 1 resolves and holds its lease for the whole test (the user is answering prompts).
    let s1 = resolve(remote.as_str(), Some("main"), &root, &opts(FRESH)).unwrap();

    // Another session on the SAME commit coexists (shared leases don't exclude each other).
    let s2 = resolve(remote.as_str(), Some("main"), &root, &opts(FRESH)).unwrap();
    assert_eq!(s2.oid, s1.oid);
    assert_eq!(s2.tree_dir, s1.tree_dir);

    // The branch moves; a new session gets a new tree WITHOUT waiting on s1's still-held lease.
    move_remote(&remote, "two");
    let s3 = resolve(remote.as_str(), Some("main"), &root, &opts(EXPIRED)).unwrap();
    assert_ne!(s3.oid, s1.oid);

    // s1, at lunch, still sees exactly what it resolved.
    assert_eq!(std::fs::read_to_string(s1.tree_dir.join("file.txt")).unwrap(), "one");
    assert_eq!(std::fs::read_to_string(s3.tree_dir.join("file.txt")).unwrap(), "two");

    drop((s1, s2, s3));
    std::fs::remove_dir_all(&root).ok();
}

/// The reaper removes an unused tree past retention, but skips one a session still holds.
#[test]
fn prune_reaps_unused_but_skips_leased() {
    let root = scratch("prune");
    let remote = init_remote(&root, "one");

    // Resolve and immediately drop the lease — the tree is now unused.
    let tree = {
        let r = resolve(remote.as_str(), Some("main"), &root, &opts(FRESH)).unwrap();
        r.tree_dir.clone()
    };
    assert!(tree.exists());
    std::thread::sleep(Duration::from_millis(30)); // let its last-use age past ZERO retention

    let stats = prune(&root, Duration::ZERO).unwrap();
    assert!(stats.removed >= 1, "an unused tree should be reaped: {stats:?}");
    assert!(!tree.exists(), "the reaped tree should be gone");

    // Now hold a lease across a prune — the tree must survive.
    let held = resolve(remote.as_str(), Some("main"), &root, &opts(FRESH)).unwrap();
    std::thread::sleep(Duration::from_millis(30));
    let stats = prune(&root, Duration::ZERO).unwrap();
    assert_eq!(stats.in_use, 1, "a leased tree must be skipped: {stats:?}");
    assert!(held.tree_dir.exists(), "a leased tree must not be reaped");

    drop(held);
    std::fs::remove_dir_all(&root).ok();
}
