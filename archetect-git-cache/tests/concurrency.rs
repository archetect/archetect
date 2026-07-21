//! Concurrent `resolve` against one cold cache doesn't corrupt it — the write lock serializes
//! clone/fetch/materialize. Without it, N threads would clone/materialize into the same dirs at once.

use std::path::Path;
use std::process::Command;
use std::thread;

use archetect_git_cache::{resolve, FetchOptions, Freshness, RefPin};
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
fn concurrent_resolves_into_one_cold_cache_do_not_corrupt() {
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

    let url = remote.to_string();
    let cache_root = root.clone();

    let handles: Vec<_> = (0..8)
        .map(|_| {
            let url = url.clone();
            let cache_root = cache_root.clone();
            thread::spawn(move || {
                let opts = FetchOptions {
                    force: false,
                    offline: false,
                    interval: std::time::Duration::from_secs(3600),
                    pin: RefPin::Infer,
                };
                resolve(&url, Some("main"), &cache_root, &opts)
            })
        })
        .collect();

    let outcomes: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();

    let mut cloned = 0;
    for outcome in &outcomes {
        let r = outcome.as_ref().expect("concurrent resolve must not error");
        assert_eq!(r.oid, head, "all threads resolve the same commit");
        assert_eq!(std::fs::read_to_string(r.tree_dir.join("file.txt")).unwrap(), "payload");
        if r.freshness == Freshness::Cloned {
            cloned += 1;
        }
    }
    assert_eq!(cloned, 1, "exactly one clone, the rest cache hits: {outcomes:?}");

    std::fs::remove_dir_all(&root).ok();
}
