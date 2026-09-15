//! The freeze class (YP6M-3788): a stalled network operation must be BOUNDED, and the eager hot
//! path (IfMissing) must never queue behind a fetch in flight. archetect-server froze on dev and
//! prd (2026-09-15) because neither held: a wedged fetch blocked forever while holding the
//! per-repo write lock, and every request queued behind it until the pod was restarted.
//!
//! Own integration file deliberately: each test binary is its own process, so the short
//! `ARCHETECT_GIT_TIMEOUT_MS` set here is what the process-wide `Once` latches.

use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

use archetect_git_cache::{resolve, FetchOptions, Freshness, PullPolicy, RefPin};
use camino::Utf8PathBuf;
use fs4::fs_std::FileExt;

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
    p.push(format!("gitcache-resilience-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}

fn opts(pull: PullPolicy) -> FetchOptions {
    FetchOptions {
        pull,
        offline: false,
        interval: Duration::from_secs(3600),
        pin: RefPin::Infer,
    }
}

/// A remote that accepts TCP and never answers — the shape of a stalled GitHub connection. Every
/// resolve against it must ERROR within the deadline budget instead of hanging forever (the old
/// behavior: libgit2 blocked indefinitely, inside the write lock).
#[test]
fn a_stalled_remote_is_bounded_not_forever() {
    std::env::set_var("ARCHETECT_GIT_TIMEOUT_MS", "3000");
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    std::thread::spawn(move || {
        let mut held = Vec::new();
        for stream in listener.incoming() {
            if let Ok(s) = stream {
                held.push(s); // accept, hold open, never respond
            }
        }
    });

    let root = scratch("stall");
    let url = format!("http://127.0.0.1:{port}/stalled.git");
    let start = Instant::now();
    let result = resolve(&url, None, &root, &opts(PullPolicy::Gated));
    let elapsed = start.elapsed();

    assert!(result.is_err(), "a silent remote cannot resolve");
    // Budget: git2 clone (3s IO timeout) + CLI fallback (http stall abort at ~3s, wall-clock kill
    // at 30s floor). Generous headroom below the old behavior of NEVER returning.
    assert!(
        elapsed < Duration::from_secs(90),
        "bounded, not frozen: took {elapsed:?}"
    );
    std::fs::remove_dir_all(&root).ok();
}

/// The stale lane: with the write lock held (exactly what a wedged refresher fetch does), an
/// IfMissing resolve with a warm cache serves the existing tree immediately instead of queueing.
#[test]
fn if_missing_serves_warm_cache_while_the_write_lock_is_held() {
    let root = scratch("stale-lane");
    // A real local remote, warmed through the normal path.
    let remote = root.join("remote");
    std::fs::create_dir_all(&remote).unwrap();
    std::fs::write(remote.join("file.txt"), "one").unwrap();
    git(&["init", "-q", "-b", "main"], remote.as_std_path());
    git(&["add", "."], remote.as_std_path());
    git(&["commit", "-q", "-m", "first"], remote.as_std_path());

    let warmed = resolve(remote.as_str(), None, &root, &opts(PullPolicy::Gated)).unwrap();

    // Hold the cross-process write lock the way a fetch in flight does. (The in-process keyed
    // mutex is per-resolve; the flock is the layer a concurrent request contends on.)
    let lock_path = std::fs::read_dir(root.join("sources").as_std_path())
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .find(|p| p.extension().map(|x| x == "lock").unwrap_or(false))
        .expect("the warm resolve created the write-lock file");
    let held = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&lock_path)
        .unwrap();
    FileExt::lock_exclusive(&held).unwrap();

    let start = Instant::now();
    let r = resolve(remote.as_str(), None, &root, &opts(PullPolicy::IfMissing)).unwrap();
    let elapsed = start.elapsed();

    assert!(
        elapsed < Duration::from_secs(2),
        "the hot path must not wait on the lock: took {elapsed:?}"
    );
    assert_eq!(r.freshness, Freshness::UpToDate { probed: false });
    assert_eq!(r.oid, warmed.oid, "the stale lane serves the warmed tree");
    assert_eq!(
        std::fs::read_to_string(r.tree_dir.join("file.txt")).unwrap(),
        "one"
    );

    let _ = FileExt::unlock(&held);
    std::fs::remove_dir_all(&root).ok();
}
