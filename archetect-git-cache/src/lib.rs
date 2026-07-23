//! A host-agnostic, **content-addressed** Git source cache built for concurrent, long-lived sessions.
//!
//! Both archetect and prova fetch Git repos (archetypes, catalogs, plugins) and render/read from
//! them — sometimes interactively, for as long as a user takes to answer prompts. The unit of
//! isolation is the resolved **commit**, not the ref (a ref moves; a commit never does):
//!
//! ```text
//! <cache_root>/
//!   sources/<repo-hash>/       bare mirror per repo URL — objects + refs, the fetch target.
//!   trees/<repo-hash>/<oid>/    immutable working tree at ONE commit, materialized from the mirror.
//!     <oid>.lease              sessions hold a SHARED flock for their lifetime; the reaper needs EXCLUSIVE.
//!     <oid>.used               last-use stamp (mtime) for retention.
//! ```
//!
//! [`resolve`] does it all under a short per-repo **write lock**: ensure the mirror, run the freshness
//! gate (TTL + `ls-remote`, silent when unchanged), resolve `ref → oid`, materialize the immutable
//! tree if absent, and take a shared **lease**. The caller renders from `tree_dir` holding the
//! returned [`Lease`] — **no lock spans the render**, because the tree can never change under it.
//! A branch that moves mid-session just resolves to a new oid → a new tree; in-flight sessions keep
//! theirs. [`prune`] quietly reaps trees unused past a retention window (skipping any still leased).
//!
//! Freshness metadata lives in the mirror's git config, keyed per-ref by a farmhash slug (refs
//! contain `/` and `.`, unsafe in git-config keys).

use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, SystemTime};

use camino::{Utf8Path, Utf8PathBuf};
use fs4::fs_std::FileExt;
use git2::Repository;
use log::trace;

mod error;
mod git;

pub use error::GitCacheError;
pub use git::ls_remote;

/// The git-config section holding per-ref freshness metadata inside a mirror.
const META_SECTION: &str = "gitcache";
/// Slug input for a source with no explicit ref (the default-branch entry). A NUL byte can't appear
/// in a real ref, so this never collides with one.
const DEFAULT_REF_SENTINEL: &str = "\0default";

/// How a [`resolve`] should treat the cache. The caller derives these from its flags/config.
#[derive(Debug, Clone)]
pub struct FetchOptions {
    /// Skip the freshness gate and always fetch (archetect's `-U`, prova's `--update`).
    pub force: bool,
    /// Never touch the network; error if the requested ref isn't already cached.
    pub offline: bool,
    /// TTL width for the freshness gate — how often a moving ref re-checks the remote.
    pub interval: Duration,
    /// Whether the requested ref can move upstream — governs whether the hash gate probes at all.
    pub pin: RefPin,
}

/// Whether a ref is expected to move upstream. Only a bare commit id is content-addressed and
/// truly immutable — it is never probed or re-fetched once cached. Everything symbolic (branches
/// AND tags) gets the hash gate: git allows any ref to move, and the ecosystem's floating-major
/// convention (`v1` tracking the latest v1.x.y) depends on tags moving. The gate keeps that cheap:
/// within the TTL, zero network; past it, one ls-remote, pulling only when the oid differs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefPin {
    /// A symbolic ref — branch, tag, or the default branch: gets the ls-remote hash gate.
    Mutable,
    /// A bare commit rev: content-addressed, never probed, never re-fetched once cached. Callers
    /// should assert this only for refs that cannot move (a rev), not for tags.
    Immutable,
    /// The caller can't tell (e.g. archetect's `url#ref`): infer from local resolution — a bare
    /// commit ⇒ immutable; a tag, a branch, or the default branch ⇒ mutable.
    Infer,
}

/// A resolved, immutable source tree ready to render/read from. Hold [`lease`](Self::lease) for as
/// long as you read `tree_dir`; dropping it lets the reaper eventually reclaim the tree.
#[derive(Debug)]
pub struct ResolvedSource {
    /// The immutable working tree at `oid`. Render/require from here — it never changes.
    pub tree_dir: Utf8PathBuf,
    /// The commit OID (hex) the source resolved to.
    pub oid: String,
    /// The concrete ref used (e.g. the resolved default branch when no ref was requested).
    pub resolved_ref: String,
    /// What happened, so the caller can message appropriately.
    pub freshness: Freshness,
    /// A shared lease on the tree — keeps the reaper from reclaiming it mid-read. Drop when done.
    pub lease: Lease,
}

/// A shared lease on a materialized tree. Held for as long as a session reads the tree; released on
/// drop. It only ever excludes the reaper (the tree is immutable), so holding it for an hour blocks
/// no other session. Backed by two layers — a shared `flock` (cross-process) and a process-global
/// refcount (in-process, since `flock` self-conflict across fds is unreliable on some platforms).
#[derive(Debug)]
pub struct Lease {
    _file: File,
    tree_dir: Utf8PathBuf,
}

impl Drop for Lease {
    fn drop(&mut self) {
        lease_decr(&self.tree_dir);
        // The shared flock releases when `_file` closes on drop.
    }
}

/// Process-global count of live leases per tree dir, so the reaper (same process) sees in-flight
/// sessions even where `flock` wouldn't self-conflict.
fn active_leases() -> &'static Mutex<HashMap<Utf8PathBuf, usize>> {
    static ACTIVE: OnceLock<Mutex<HashMap<Utf8PathBuf, usize>>> = OnceLock::new();
    ACTIVE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn lease_incr(tree_dir: &Utf8Path) {
    let mut map = active_leases().lock().unwrap_or_else(|p| p.into_inner());
    *map.entry(tree_dir.to_path_buf()).or_insert(0) += 1;
}

fn lease_decr(tree_dir: &Utf8Path) {
    let mut map = active_leases().lock().unwrap_or_else(|p| p.into_inner());
    if let Some(count) = map.get_mut(tree_dir) {
        *count -= 1;
        if *count == 0 {
            map.remove(tree_dir);
        }
    }
}

fn lease_active(tree_dir: &Utf8Path) -> bool {
    let map = active_leases().lock().unwrap_or_else(|p| p.into_inner());
    map.get(tree_dir).copied().unwrap_or(0) > 0
}

/// What a [`resolve`] did — the caller's cue for whether (and what) to print.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Freshness {
    /// The mirror was absent; a fresh clone happened.
    Cloned,
    /// A fetch happened (hash gate differed, or force / missing-ref). Announce it.
    Updated,
    /// No fetch — the cache was used as-is. `probed` is true when the hash gate ran a network probe
    /// that matched (TTL had expired but the remote hadn't moved); false when the TTL was still
    /// fresh (or offline), so no network happened. Either way the caller should stay silent.
    UpToDate { probed: bool },
}

/// What a [`prune`] swept.
#[derive(Debug, Default, Clone, Copy)]
pub struct PruneStats {
    /// Trees removed (past retention, not leased).
    pub removed: usize,
    /// Trees kept (still within retention).
    pub kept: usize,
    /// Trees eligible by age but skipped because a session still holds them.
    pub in_use: usize,
}

/// Resolve `url`/`gitref` to an immutable tree under `cache_root`: ensure the bare mirror, run the
/// freshness gate, materialize the commit's tree if absent, and take a session lease. See the module
/// docs. The write lock is held only for this (short) call, never across the caller's render.
pub fn resolve(
    url: &str,
    gitref: Option<&str>,
    cache_root: &Utf8Path,
    opts: &FetchOptions,
) -> Result<ResolvedSource, GitCacheError> {
    let hash = repo_hash(url);
    let sources_dir = cache_root.join("sources").join(&hash);
    let trees_root = cache_root.join("trees").join(&hash);
    with_write_lock(&sources_dir, || {
        resolve_locked(url, gitref, &sources_dir, &trees_root, opts)
    })
}

fn resolve_locked(
    url: &str,
    gitref: Option<&str>,
    sources_dir: &Utf8Path,
    trees_root: &Utf8Path,
    opts: &FetchOptions,
) -> Result<ResolvedSource, GitCacheError> {
    let now = now_ms();
    let slug = slug_for(gitref);

    // ── Ensure the mirror + decide freshness ────────────────────────────────────────────────
    let freshness;
    let mut did_fetch = false;
    if !sources_dir.exists() {
        if opts.offline {
            return Err(GitCacheError::OfflineAndNotCached(url.to_string()));
        }
        git::clone_mirror(url, sources_dir)?;
        did_fetch = true;
        freshness = Freshness::Cloned;
    } else {
        let repo = Repository::open_bare(sources_dir.as_std_path())?;
        let mut probed_match = false;
        let do_fetch = if opts.force {
            true
        } else if opts.offline {
            false
        } else if !repo_has_any_ref(&repo) {
            true // mirror was cloned empty; the remote likely has content now
        } else if gitref.is_some_and(|g| !ref_exists_local(&repo, g)) {
            true // requested ref isn't in the mirror — fetch to obtain it
        } else {
            let cfg = meta_config(sources_dir)?;
            let meta = read_meta(&cfg, &slug);
            match meta.checked_at_ms {
                None => true,
                Some(ts) if now.saturating_sub(ts) <= interval_ms(opts) => false, // TTL fresh: no network
                Some(_) => {
                    if is_immutable(opts.pin, &repo, gitref) {
                        // A bare rev cannot move — don't probe; just reset the TTL and stay silent.
                        let mut cfg = meta_config(sources_dir)?;
                        refresh_checked_at(&mut cfg, &slug, now)?;
                        false
                    } else {
                        // Hash gate: the one place we probe the remote.
                        let remote_oid = git::ls_remote(url, gitref.unwrap_or("HEAD"))?;
                        match (remote_oid, meta.oid.as_deref()) {
                            (Some(remote), Some(stored)) if remote == stored => {
                                let mut cfg = meta_config(sources_dir)?;
                                refresh_checked_at(&mut cfg, &slug, now)?;
                                probed_match = true;
                                false
                            }
                            _ => true, // differ / no baseline / dropped ref → fetch
                        }
                    }
                }
            }
        };

        if do_fetch {
            git::fetch_repo(sources_dir)?;
            did_fetch = true;
        }
        freshness = if do_fetch {
            Freshness::Updated
        } else {
            Freshness::UpToDate { probed: probed_match }
        };
    }

    // ── Resolve the target commit from the (possibly freshly fetched) mirror ─────────────────
    let repo = Repository::open_bare(sources_dir.as_std_path())?;
    if let Some(g) = gitref {
        if !ref_exists_local(&repo, g) {
            return Err(if opts.offline {
                GitCacheError::RefNotCachedOffline(g.to_string())
            } else {
                GitCacheError::Remote(format!("ref `{g}` not found on remote {url}"))
            });
        }
    }
    let (oid, resolved_ref) = resolve_oid(&repo, gitref)?;

    // ── Record freshness metadata ───────────────────────────────────────────────────────────
    {
        let mut cfg = meta_config(sources_dir)?;
        if did_fetch {
            write_meta(&mut cfg, &slug, &resolved_ref, &oid, now)?;
        } else {
            ensure_baseline(&mut cfg, &slug, &resolved_ref, &oid)?;
        }
    }

    // ── Materialize the immutable tree (once) + lease it for the session ─────────────────────
    let tree_dir = trees_root.join(&oid);
    if !tree_dir.exists() {
        materialize_atomic(sources_dir, &oid, trees_root, &tree_dir)?;
    }
    touch_used(trees_root, &oid, now)?;
    let lease = acquire_shared_lease(trees_root, &oid)?;

    trace!("resolved {url} {resolved_ref} @ {oid} -> {tree_dir}");
    Ok(ResolvedSource {
        tree_dir,
        oid,
        resolved_ref,
        freshness,
        lease,
    })
}

/// Drop the freshness metadata for one ref of `url` so the next [`resolve`] re-probes/re-fetches it.
/// `gitref = None` targets the default-branch entry. No-op if the mirror doesn't exist.
pub fn invalidate(cache_root: &Utf8Path, url: &str, gitref: Option<&str>) -> Result<(), GitCacheError> {
    let sources_dir = cache_root.join("sources").join(repo_hash(url));
    if !sources_dir.exists() {
        return Ok(());
    }
    with_write_lock(&sources_dir, || {
        let mut cfg = meta_config(&sources_dir)?;
        remove_slug(&mut cfg, &slug_for(gitref));
        Ok(())
    })
}

/// Drop *all* freshness metadata for `url` (backs `cache invalidate`). No-op if the mirror is absent.
pub fn invalidate_all(cache_root: &Utf8Path, url: &str) -> Result<(), GitCacheError> {
    let sources_dir = cache_root.join("sources").join(repo_hash(url));
    if !sources_dir.exists() {
        return Ok(());
    }
    with_write_lock(&sources_dir, || {
        let mut cfg = meta_config(&sources_dir)?;
        let mut names = Vec::new();
        {
            let entries = cfg.entries(Some(&format!("^{META_SECTION}\\.")))?;
            entries.for_each(|entry| {
                if let Some(name) = entry.name() {
                    names.push(name.to_string());
                }
            })?;
        }
        for name in names {
            let _ = cfg.remove(&name);
        }
        Ok(())
    })
}

/// Quietly reap materialized trees under `cache_root` unused longer than `retention` — but only ones
/// no session still holds (a non-blocking exclusive `flock` on `<oid>.lease` proves it). Crash-leftover
/// `.tmp-*` dirs are always removed. Runs per-repo under the write lock so it never races a resolve.
pub fn prune(cache_root: &Utf8Path, retention: Duration) -> Result<PruneStats, GitCacheError> {
    let mut stats = PruneStats::default();
    let trees = cache_root.join("trees");
    if !trees.exists() {
        return Ok(stats);
    }
    let now = SystemTime::now();
    for entry in std::fs::read_dir(trees.as_std_path())? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let repo_trees = Utf8PathBuf::from_path_buf(entry.path())
            .map_err(|p| GitCacheError::Remote(format!("non-UTF-8 cache path: {}", p.display())))?;
        let hash = repo_trees.file_name().unwrap_or_default().to_string();
        let sources_dir = cache_root.join("sources").join(&hash);
        with_write_lock(&sources_dir, || {
            prune_repo_trees(&repo_trees, retention, now, &mut stats)
        })?;
    }
    Ok(stats)
}

fn prune_repo_trees(
    repo_trees: &Utf8Path,
    retention: Duration,
    now: SystemTime,
    stats: &mut PruneStats,
) -> Result<(), GitCacheError> {
    for entry in std::fs::read_dir(repo_trees.as_std_path())? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        let path = repo_trees.join(&name);
        // Crash-leftover partial materializations — always safe to remove.
        if name.starts_with(".tmp-") {
            let _ = std::fs::remove_dir_all(path.as_std_path());
            continue;
        }
        // Only <oid>/ dirs are trees; the sidecar <oid>.lease / <oid>.used files ride with them.
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let oid = name;
        let used = repo_trees.join(format!("{oid}.used"));
        let mtime = std::fs::metadata(used.as_std_path())
            .and_then(|m| m.modified())
            .or_else(|_| std::fs::metadata(path.as_std_path()).and_then(|m| m.modified()))
            .unwrap_or(now);
        if now.duration_since(mtime).unwrap_or_default() <= retention {
            stats.kept += 1;
            continue;
        }
        // Eligible by age — but only reap if no session holds the lease. Check the in-process
        // registry first (catches same-process sessions regardless of flock self-conflict quirks),
        // then the cross-process advisory lock.
        if lease_active(&path) {
            stats.in_use += 1;
            continue;
        }
        let lease_path = repo_trees.join(format!("{oid}.lease"));
        let lease = OpenOptions::new()
            .create(true)
            .append(true)
            .open(lease_path.as_std_path());
        match lease {
            Ok(file) if FileExt::try_lock_exclusive(&file).is_ok() => {
                let _ = std::fs::remove_dir_all(path.as_std_path());
                let _ = std::fs::remove_file(used.as_std_path());
                let _ = FileExt::unlock(&file);
                drop(file);
                let _ = std::fs::remove_file(lease_path.as_std_path());
                stats.removed += 1;
            }
            _ => stats.in_use += 1,
        }
    }
    Ok(())
}

// ── materialization ─────────────────────────────────────────────────────────────────────────────

/// Materialize `oid`'s tree into `trees_root/<oid>/` via a temp dir + atomic rename, so a crash never
/// leaves a half-written dir that looks complete. Called under the write lock, so no concurrent
/// materialize of the same oid.
fn materialize_atomic(
    mirror_dir: &Utf8Path,
    oid: &str,
    trees_root: &Utf8Path,
    tree_dir: &Utf8Path,
) -> Result<(), GitCacheError> {
    std::fs::create_dir_all(trees_root.as_std_path())?;
    let tmp = trees_root.join(format!(".tmp-{oid}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(tmp.as_std_path());
    git::materialize(mirror_dir, oid, &tmp)?;
    match std::fs::rename(tmp.as_std_path(), tree_dir.as_std_path()) {
        Ok(()) => Ok(()),
        Err(_) if tree_dir.exists() => {
            let _ = std::fs::remove_dir_all(tmp.as_std_path());
            Ok(())
        }
        Err(e) => Err(e.into()),
    }
}

fn touch_used(trees_root: &Utf8Path, oid: &str, now_ms: i64) -> Result<(), GitCacheError> {
    std::fs::create_dir_all(trees_root.as_std_path())?;
    std::fs::write(trees_root.join(format!("{oid}.used")).as_std_path(), now_ms.to_string())?;
    Ok(())
}

fn acquire_shared_lease(trees_root: &Utf8Path, oid: &str) -> Result<Lease, GitCacheError> {
    let tree_dir = trees_root.join(oid);
    lease_incr(&tree_dir);
    let path = trees_root.join(format!("{oid}.lease"));
    let file = OpenOptions::new().create(true).append(true).open(path.as_std_path())?;
    if let Err(err) = FileExt::lock_shared(&file) {
        lease_decr(&tree_dir); // don't leak the refcount if the flock fails
        return Err(err.into());
    }
    Ok(Lease { _file: file, tree_dir })
}

// ── metadata (git config in the bare mirror) ────────────────────────────────────────────────────

struct Meta {
    oid: Option<String>,
    checked_at_ms: Option<i64>,
}

/// Open the mirror's own config (`<mirror>/config`) for reading/writing our keys.
fn meta_config(mirror_dir: &Utf8Path) -> Result<git2::Config, GitCacheError> {
    Ok(git2::Config::open(mirror_dir.join("config").as_std_path())?)
}

/// A farmhash hex slug of the ref — a git-config-safe key.
fn slug_for(gitref: Option<&str>) -> String {
    let key = gitref.unwrap_or(DEFAULT_REF_SENTINEL);
    format!("{:016x}", farmhash::fingerprint64(key.as_bytes()))
}

/// A farmhash hex key of the repo URL (lightly normalized) — the `sources/`/`trees/` bucket. Both
/// tools produce the same key for the same URL, so they can share a cache.
fn repo_hash(url: &str) -> String {
    let norm = url.trim_end_matches('/');
    let norm = norm.strip_suffix(".git").unwrap_or(norm);
    format!("{:016x}", farmhash::fingerprint64(norm.as_bytes()))
}

fn key(slug: &str, name: &str) -> String {
    format!("{META_SECTION}.{slug}.{name}")
}

fn read_meta(cfg: &git2::Config, slug: &str) -> Meta {
    Meta {
        oid: cfg.get_string(&key(slug, "oid")).ok(),
        checked_at_ms: cfg.get_i64(&key(slug, "checkedatms")).ok(),
    }
}

fn write_meta(
    cfg: &mut git2::Config,
    slug: &str,
    resolved_ref: &str,
    oid: &str,
    now_ms: i64,
) -> Result<(), GitCacheError> {
    cfg.set_str(&key(slug, "ref"), resolved_ref)?;
    cfg.set_str(&key(slug, "oid"), oid)?;
    cfg.set_i64(&key(slug, "checkedatms"), now_ms)?;
    Ok(())
}

fn refresh_checked_at(cfg: &mut git2::Config, slug: &str, now_ms: i64) -> Result<(), GitCacheError> {
    cfg.set_i64(&key(slug, "checkedatms"), now_ms)?;
    Ok(())
}

/// Write an OID baseline only if none is recorded yet — heals a cache that has a timestamp but no
/// baseline without clobbering a real one or advancing the timestamp.
fn ensure_baseline(cfg: &mut git2::Config, slug: &str, resolved_ref: &str, oid: &str) -> Result<(), GitCacheError> {
    if cfg.get_string(&key(slug, "oid")).is_err() {
        cfg.set_str(&key(slug, "ref"), resolved_ref)?;
        cfg.set_str(&key(slug, "oid"), oid)?;
    }
    Ok(())
}

fn remove_slug(cfg: &mut git2::Config, slug: &str) {
    for name in ["ref", "oid", "checkedatms"] {
        let _ = cfg.remove(&key(slug, name));
    }
}

// ── ref resolution (bare-mirror layout: branches at refs/heads, tags at refs/tags) ───────────────

fn is_branch(repo: &Repository, gitref: &str) -> bool {
    repo.find_reference(&format!("refs/heads/{gitref}")).is_ok()
}

fn is_tag(repo: &Repository, gitref: &str) -> bool {
    repo.find_reference(&format!("refs/tags/{gitref}")).is_ok()
}

/// Is `gitref` resolvable in the mirror at all (branch, tag, or commit)?
fn ref_exists_local(repo: &Repository, gitref: &str) -> bool {
    repo.revparse_single(gitref).is_ok() || repo.revparse_single(&format!("{gitref}^{{commit}}")).is_ok()
}

fn repo_has_any_ref(repo: &Repository) -> bool {
    match repo.references() {
        Ok(mut refs) => refs.next().is_some(),
        Err(_) => false,
    }
}

/// The mirror's default branch (its HEAD symref), falling back to main/master.
fn default_branch(repo: &Repository) -> Result<String, GitCacheError> {
    if let Ok(head) = repo.head() {
        if let Some(shorthand) = head.shorthand() {
            if is_branch(repo, shorthand) {
                return Ok(shorthand.to_string());
            }
        }
    }
    for candidate in ["main", "master"] {
        if is_branch(repo, candidate) {
            return Ok(candidate.to_string());
        }
    }
    Err(GitCacheError::NoDefaultBranch)
}

/// Under `RefPin::Infer`, decide immutability from local resolution. Only a bare commit rev is
/// content-addressed and cannot move; a tag is a symbolic ref exactly like a branch — git allows
/// it to move, and floating majors (`v1` tracking v1.x.y) rely on that — so every symbolic ref
/// takes the hash gate.
fn is_immutable(pin: RefPin, repo: &Repository, gitref: Option<&str>) -> bool {
    match pin {
        RefPin::Immutable => true,
        RefPin::Mutable => false,
        RefPin::Infer => match gitref {
            None => false,
            Some(g) => {
                !is_tag(repo, g)
                    && !is_branch(repo, g)
                    && repo.revparse_single(&format!("{g}^{{commit}}")).is_ok()
            }
        },
    }
}

/// Resolve `gitref` (or the default branch when `None`) to its peeled commit OID + the ref name used.
/// No checkout — content-addressing checks out per-commit into `trees/`.
fn resolve_oid(repo: &Repository, gitref: Option<&str>) -> Result<(String, String), GitCacheError> {
    let resolved_ref = match gitref {
        Some(g) => g.to_string(),
        None => default_branch(repo)?,
    };
    let object = repo.revparse_single(&resolved_ref)?;
    let commit = object.peel_to_commit()?;
    Ok((commit.id().to_string(), resolved_ref))
}

// ── write lock (short, per-repo — reused for resolve and prune) ──────────────────────────────────

/// Run `f` holding the exclusive per-repo write lock: an in-process keyed mutex (many `Archetect`
/// instances / server requests / threads in one process) plus a cross-process advisory `flock` on a
/// sibling `<name>.lock` (separate OS processes). Both are needed — `flock` is process-wide, so it
/// doesn't serialize threads within a process; the mutex does. Held only for fetch/resolve/materialize
/// or a prune sweep, never across a render.
fn with_write_lock<T>(
    sources_dir: &Utf8Path,
    f: impl FnOnce() -> Result<T, GitCacheError>,
) -> Result<T, GitCacheError> {
    let mutex = keyed_mutex(sources_dir);
    let _in_process = mutex.lock().unwrap_or_else(|poisoned| poisoned.into_inner());

    let lock_file = acquire_exclusive_lock(sources_dir)?;
    let result = f();
    let _ = FileExt::unlock(&lock_file);
    result
}

fn keyed_mutex(sources_dir: &Utf8Path) -> Arc<Mutex<()>> {
    static LOCKS: OnceLock<Mutex<HashMap<Utf8PathBuf, Arc<Mutex<()>>>>> = OnceLock::new();
    let registry = LOCKS.get_or_init(|| Mutex::new(HashMap::new()));
    let mut map = registry.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    map.entry(sources_dir.to_path_buf())
        .or_insert_with(|| Arc::new(Mutex::new(())))
        .clone()
}

/// Open (creating if needed) the sibling `<name>.lock` for `sources_dir` and take an exclusive
/// advisory lock. The lock is a sibling so it exists before the mirror is cloned.
fn acquire_exclusive_lock(sources_dir: &Utf8Path) -> Result<File, GitCacheError> {
    if let Some(parent) = sources_dir.parent() {
        std::fs::create_dir_all(parent.as_std_path())?;
    }
    let name = sources_dir.file_name().unwrap_or("cache");
    let lock_path = match sources_dir.parent() {
        Some(parent) => parent.join(format!("{name}.lock")),
        None => Utf8PathBuf::from(format!("{sources_dir}.lock")),
    };
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(lock_path.as_std_path())?;
    FileExt::lock_exclusive(&file)?;
    Ok(file)
}

// ── small helpers ───────────────────────────────────────────────────────────────────────────────

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

fn interval_ms(opts: &FetchOptions) -> i64 {
    i64::try_from(opts.interval.as_millis()).unwrap_or(i64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_is_stable_hex_and_default_differs() {
        assert_eq!(slug_for(Some("main")), slug_for(Some("main")));
        assert_ne!(slug_for(Some("main")), slug_for(None));
        assert_eq!(slug_for(Some("v1")).len(), 16);
        assert!(slug_for(Some("v1")).chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn repo_hash_normalizes_git_suffix_and_slash() {
        assert_eq!(repo_hash("https://x/y"), repo_hash("https://x/y.git"));
        assert_eq!(repo_hash("https://x/y"), repo_hash("https://x/y/"));
        assert_ne!(repo_hash("https://x/y"), repo_hash("https://x/z"));
    }
}
