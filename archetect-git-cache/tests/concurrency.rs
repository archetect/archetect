//! Proof that concurrent `fetch()` calls against the *same* cache dir don't corrupt it — the write
//! lock serializes clone/fetch/checkout. The dangerous case is a cold cache: without the lock, N
//! threads would all take the clone path into one dir at once (and the git2→CLI fallback would
//! `remove_dir_all` under each other). With the lock, one clones and the rest reuse the result.
//!
//! This exercises the in-process keyed mutex (all threads share one process); the sibling file lock
//! is taken on the same path, so the two layers run together. Cross-process behavior rides on the
//! same `flock` and isn't unit-tested here (it needs separate OS processes).

use std::path::Path;
use std::process::Command;
use std::thread;

use archetect_git_cache::{fetch, FetchOptions, Freshness, RefPin};
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

#[test]
fn concurrent_fetches_into_one_cold_cache_do_not_corrupt() {
    let root = {
        let mut p = Utf8PathBuf::from_path_buf(std::env::temp_dir()).unwrap();
        p.push(format!("gitcache-concurrency-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    };
    let remote = root.join("remote");
    std::fs::create_dir_all(&remote).unwrap();
    std::fs::write(remote.join("file.txt"), "payload").unwrap();
    git(&["init", "-q", "-b", "main"], remote.as_std_path());
    git(&["add", "."], remote.as_std_path());
    git(&["commit", "-q", "-m", "one"], remote.as_std_path());
    let head = git_out(&["rev-parse", "HEAD"], remote.as_std_path());

    // The one cache dir every thread races to populate.
    let cache = root.join("cache");
    let url = remote.to_string();

    let handles: Vec<_> = (0..8)
        .map(|_| {
            let url = url.clone();
            let cache = cache.clone();
            thread::spawn(move || {
                let opts = FetchOptions {
                    force: false,
                    offline: false,
                    interval: std::time::Duration::from_secs(3600),
                    pin: RefPin::Infer,
                };
                fetch(&url, None, &cache, &opts)
            })
        })
        .collect();

    let outcomes: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();

    // Every thread succeeded, and all agree on the checked-out commit — no torn clone/checkout.
    let mut cloned = 0;
    for outcome in &outcomes {
        let outcome = outcome.as_ref().expect("concurrent fetch must not error");
        assert_eq!(outcome.resolved_oid, head, "all threads resolve the same commit");
        if outcome.freshness == Freshness::Cloned {
            cloned += 1;
        }
    }
    // Exactly one thread did the clone; the rest reused the cache (proof they serialized rather than
    // all cloning into the same dir).
    assert_eq!(cloned, 1, "exactly one clone, the rest cache hits: {outcomes:?}");

    // The working tree is intact and correct.
    let content = std::fs::read_to_string(cache.join("file.txt")).unwrap();
    assert_eq!(content, "payload");

    std::fs::remove_dir_all(&root).ok();
}
