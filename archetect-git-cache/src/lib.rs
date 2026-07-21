//! A host-agnostic Git source cache with a **two-gate freshness check**.
//!
//! Both archetect and prova fetch Git repos (archetypes, catalogs, plugins) and cache them locally.
//! This crate is the one shared implementation of *when* a cached repo is refreshed:
//!
//! 1. **TTL gate** (local, free): within `interval` of the last check, use the cache — zero network.
//! 2. **Hash gate** (network, cheap): once the TTL expires, `git ls-remote` the ref and compare its
//!    OID to the one recorded at last fetch.
//!    - **Match** → nothing changed. Refresh the timestamp, do *not* fetch, and report `UpToDate`
//!      (the caller stays silent — the cache is already current).
//!    - **Differ** → full fetch + checkout, record the new OID, report `Updated` (the caller says so).
//! 3. `force` skips both gates. `offline` never touches the network.
//!
//! ## What the crate owns vs. the caller
//!
//! The **caller** owns directory layout and user-facing messages; it passes a `cache_path` (the repo
//! dir where `.git` lives) per call. The **crate** owns that one directory: clone/fetch/ls-remote,
//! the gates, the metadata, and the checkout. This keeps archetect's one-dir-per-repo `farmhash`
//! scheme and prova's one-dir-per-`(url,ref)` scheme both expressible without the crate knowing
//! either — the metadata is keyed per-ref within a dir, so a multiplexed dir (archetect) and a
//! single-ref dir (prova) are the same code.
//!
//! Freshness metadata lives in the cache repo's own `.git/config` (never touched by the detached
//! checkout), under `[gitcache "<slug>"]` where `<slug>` is a farmhash of the ref (refs contain `/`
//! and `.`, which are unsafe in git-config keys). See [`fetch`].

use camino::{Utf8Path, Utf8PathBuf};
use git2::Repository;
use log::trace;

mod error;
mod git;

pub use error::GitCacheError;
// Low-level fetch-class ops, re-exported so hosts can drop their own copies.
pub use git::{clone, fetch_repo, ls_remote};

/// The git-config section that holds per-ref freshness metadata inside a cache repo.
const META_SECTION: &str = "gitcache";
/// Slug input for a source with no explicit ref (the default-branch entry). A NUL byte can't appear
/// in a real ref, so this never collides with one.
const DEFAULT_REF_SENTINEL: &str = "\0default";

/// How a [`fetch`] should treat the cache. The caller derives these from its flags/config.
#[derive(Debug, Clone)]
pub struct FetchOptions {
    /// Skip both gates and always fetch (archetect's `-U`, prova's `--update`).
    pub force: bool,
    /// Never touch the network; error if the requested ref isn't already cached.
    pub offline: bool,
    /// TTL width for the freshness gate.
    pub interval: std::time::Duration,
    /// Whether the requested ref can move upstream — governs whether the hash gate probes at all.
    pub pin: RefPin,
}

/// Whether a ref is expected to move upstream. An immutable ref (a tag or a commit) is never probed
/// or re-fetched once cached; a mutable ref (a branch, or the default branch) gets the hash gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefPin {
    /// A branch or the default branch: gets the ls-remote hash gate.
    Mutable,
    /// A tag or commit rev: never probed, never re-fetched once cached.
    Immutable,
    /// The caller can't tell (e.g. archetect's `url#ref`): infer from local resolution — a tag or a
    /// bare commit ⇒ immutable, a branch ⇒ mutable, the default branch ⇒ mutable.
    Infer,
}

/// The result of a [`fetch`] or [`checkout`].
#[derive(Debug, Clone)]
pub struct FetchOutcome {
    /// The checkout directory (== the `cache_path` passed in). The caller renders/requires from here.
    pub checkout_dir: Utf8PathBuf,
    /// What happened, so the caller can message appropriately.
    pub freshness: Freshness,
    /// The commit OID (hex) now checked out — also the baseline stored for the next hash gate.
    pub resolved_oid: String,
    /// The concrete ref used (e.g. the resolved default branch when no ref was requested).
    pub resolved_ref: String,
}

/// What a [`fetch`] did — the caller's cue for whether (and what) to print.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Freshness {
    /// The cache was absent; a fresh clone happened.
    Cloned,
    /// A fetch happened (hash gate differed, or force/empty-recovery/missing-ref). Announce it.
    Updated,
    /// No fetch — the cache was used as-is. `probed` is true when the hash gate ran a network probe
    /// that matched (TTL had expired but the remote hadn't moved); false when the TTL was still
    /// fresh (or offline), so no network happened. Either way the caller should stay silent.
    UpToDate { probed: bool },
}

/// Clone-or-open the repo at `cache_path`, apply the two-gate freshness check, fetch if needed,
/// check out `gitref` (detached HEAD), and record freshness metadata. See the module docs.
pub fn fetch(
    url: &str,
    gitref: Option<&str>,
    cache_path: &Utf8Path,
    opts: &FetchOptions,
) -> Result<FetchOutcome, GitCacheError> {
    let now = now_ms();
    let slug = slug_for(gitref);

    // ── Gate 0: cache absent ────────────────────────────────────────────────────────────────
    if !cache_path.exists() {
        if opts.offline {
            return Err(GitCacheError::OfflineAndNotCached(url.to_string()));
        }
        git::clone(url, cache_path)?;
        let repo = Repository::open(cache_path.as_std_path())?;
        let (oid, resolved_ref) = checkout_resolve(&repo, gitref)?;
        let mut cfg = meta_config(cache_path)?;
        write_meta(&mut cfg, &slug, &resolved_ref, &oid, now)?;
        return Ok(outcome(cache_path, Freshness::Cloned, oid, resolved_ref));
    }

    let repo = Repository::open(cache_path.as_std_path())?;

    // ── Decide whether to fetch ─────────────────────────────────────────────────────────────
    let mut probed_match = false;
    let do_fetch = if opts.force {
        true
    } else if opts.offline {
        false
    } else if !repo_has_any_branch(&repo) {
        // Empty-clone recovery: the repo was cloned from an empty remote that likely has content now.
        true
    } else if gitref.is_some_and(|g| !ref_exists_local(&repo, g)) {
        // Requested ref isn't in the cache — must fetch or we can't check it out.
        true
    } else {
        let cfg = meta_config(cache_path)?;
        let meta = read_meta(&cfg, &slug);
        match meta.checked_at_ms {
            // Never recorded (fresh scheme, or a migrated archetect cache) — fetch once to seed it.
            None => true,
            // ── Gate 1: TTL still fresh ── zero network.
            Some(ts) if now.saturating_sub(ts) <= interval_ms(opts) => false,
            // TTL expired.
            Some(_) => {
                if is_immutable(opts.pin, &repo, gitref) {
                    // A tag/rev never moves — don't even probe; just reset the TTL and stay silent.
                    let mut cfg = meta_config(cache_path)?;
                    refresh_checked_at(&mut cfg, &slug, now)?;
                    false
                } else {
                    // ── Gate 2: hash gate ── the one place we probe the remote.
                    let probe_ref = gitref.unwrap_or("HEAD");
                    let remote_oid = git::ls_remote(url, probe_ref)?;
                    match (remote_oid, meta.oid.as_deref()) {
                        (Some(remote), Some(stored)) if remote == stored => {
                            // Nothing changed: refresh the TTL, no fetch, stay silent.
                            let mut cfg = meta_config(cache_path)?;
                            refresh_checked_at(&mut cfg, &slug, now)?;
                            probed_match = true;
                            false
                        }
                        // Differ, or no stored baseline, or the remote dropped the ref — fetch.
                        _ => true,
                    }
                }
            }
        }
    };

    if do_fetch {
        git::fetch_repo(cache_path)?;
    }

    // ── Checkout (local; re-open so post-fetch refs are visible) ────────────────────────────
    let repo = Repository::open(cache_path.as_std_path())?;
    if let Some(g) = gitref {
        if !ref_exists_local(&repo, g) {
            return Err(if opts.offline {
                GitCacheError::RefNotCachedOffline(g.to_string())
            } else {
                GitCacheError::Remote(format!("ref `{g}` not found on remote {url}"))
            });
        }
    }
    let (oid, resolved_ref) = checkout_resolve(&repo, gitref)?;

    if do_fetch {
        let mut cfg = meta_config(cache_path)?;
        write_meta(&mut cfg, &slug, &resolved_ref, &oid, now)?;
        Ok(outcome(cache_path, Freshness::Updated, oid, resolved_ref))
    } else {
        // Heal a cache that has a TTL but no baseline OID yet (migrated, or first cache hit).
        let mut cfg = meta_config(cache_path)?;
        ensure_baseline(&mut cfg, &slug, &resolved_ref, &oid)?;
        Ok(outcome(
            cache_path,
            Freshness::UpToDate { probed: probed_match },
            oid,
            resolved_ref,
        ))
    }
}

/// Local-only re-checkout of `gitref` from an already-populated `cache_path`. No network, no gates,
/// no metadata write. Callers with a per-run dedup guard use this for the 2nd..Nth reference to a
/// repo the first [`fetch`] already populated (that fetch pulled every ref via `--tags`).
pub fn checkout(gitref: Option<&str>, cache_path: &Utf8Path) -> Result<FetchOutcome, GitCacheError> {
    let repo = Repository::open(cache_path.as_std_path())?;
    if let Some(g) = gitref {
        if !ref_exists_local(&repo, g) {
            return Err(GitCacheError::RefNotCachedOffline(g.to_string()));
        }
    }
    let (oid, resolved_ref) = checkout_resolve(&repo, gitref)?;
    Ok(outcome(
        cache_path,
        Freshness::UpToDate { probed: false },
        oid,
        resolved_ref,
    ))
}

/// Drop the freshness metadata for one ref so the next [`fetch`] re-probes/re-fetches it.
/// `gitref = None` targets the default-branch entry.
pub fn invalidate(cache_path: &Utf8Path, gitref: Option<&str>) -> Result<(), GitCacheError> {
    let mut cfg = meta_config(cache_path)?;
    remove_slug(&mut cfg, &slug_for(gitref));
    Ok(())
}

/// Drop *all* freshness metadata in the cache repo (backs `archetect cache invalidate`). Also clears
/// archetect's legacy `archetect.pulled` key if present, so an old cache converges cleanly.
pub fn invalidate_all(cache_path: &Utf8Path) -> Result<(), GitCacheError> {
    let mut cfg = meta_config(cache_path)?;
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
    let _ = cfg.remove("archetect.pulled");
    Ok(())
}

// ── metadata (git config in the cache repo) ─────────────────────────────────────────────────────

struct Meta {
    oid: Option<String>,
    checked_at_ms: Option<i64>,
}

/// Open the cache repo's own config file (`<cache_path>/.git/config`) for reading/writing our keys.
/// Opening the file directly (not `repo.config()`) guarantees writes land in the repo config, not a
/// global one.
fn meta_config(cache_path: &Utf8Path) -> Result<git2::Config, GitCacheError> {
    let path = cache_path.join(".git").join("config");
    Ok(git2::Config::open(path.as_std_path())?)
}

/// A farmhash hex slug of the ref — a git-config-safe key (refs contain `/` and `.`, which aren't).
fn slug_for(gitref: Option<&str>) -> String {
    let key = gitref.unwrap_or(DEFAULT_REF_SENTINEL);
    format!("{:016x}", farmhash::fingerprint64(key.as_bytes()))
}

fn key(slug: &str, name: &str) -> String {
    format!("{META_SECTION}.{slug}.{name}")
}

fn read_meta(cfg: &git2::Config, slug: &str) -> Meta {
    Meta {
        oid: cfg.get_string(&key(slug, "oid")).ok(),
        // git lowercases config variable names; read the stored form.
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

/// Write an OID baseline (and ref) only if none is recorded yet — heals migrated/first-hit caches
/// without clobbering a real baseline or advancing the timestamp.
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

// ── ref resolution & checkout (ported from archetect's source.rs) ───────────────────────────────

/// Does `refs/remotes/origin/<gitref>` exist? (i.e. is this a fetched branch)
fn is_branch(repo: &Repository, gitref: &str) -> bool {
    repo.find_reference(&format!("refs/remotes/origin/{gitref}")).is_ok()
}

/// Does `refs/tags/<gitref>` exist?
fn is_tag(repo: &Repository, gitref: &str) -> bool {
    repo.find_reference(&format!("refs/tags/{gitref}")).is_ok()
}

/// Is `gitref` a tag or a direct commit hash?
fn is_tag_or_commit(repo: &Repository, gitref: &str) -> bool {
    is_tag(repo, gitref) || repo.revparse_single(&format!("{gitref}^{{commit}}")).is_ok()
}

/// Is `gitref` resolvable locally at all (branch, tag, or commit)?
fn ref_exists_local(repo: &Repository, gitref: &str) -> bool {
    is_branch(repo, gitref) || is_tag_or_commit(repo, gitref)
}

fn find_default_branch(repo: &Repository) -> Result<String, GitCacheError> {
    for candidate in ["main", "master"] {
        if is_branch(repo, candidate) {
            return Ok(candidate.to_string());
        }
    }
    Err(GitCacheError::NoDefaultBranch)
}

fn repo_has_any_branch(repo: &Repository) -> bool {
    match repo.branches(None) {
        Ok(mut iter) => iter.next().is_some(),
        Err(_) => false,
    }
}

/// Under `RefPin::Infer`, decide immutability from local resolution: a tag or a bare commit never
/// moves; a branch (or the default branch) does.
fn is_immutable(pin: RefPin, repo: &Repository, gitref: Option<&str>) -> bool {
    match pin {
        RefPin::Immutable => true,
        RefPin::Mutable => false,
        RefPin::Infer => match gitref {
            None => false, // the default branch moves
            Some(g) => {
                if is_tag(repo, g) {
                    true
                } else if is_branch(repo, g) {
                    false
                } else {
                    // Not a tag, not a branch: an exact commit hash ⇒ immutable; unknown ⇒ mutable.
                    repo.revparse_single(&format!("{g}^{{commit}}")).is_ok()
                }
            }
        },
    }
}

/// Detached-HEAD checkout of `gitref` (or the default branch when `None`). Returns the peeled commit
/// OID (the stable baseline for the hash gate) and the concrete ref name used.
fn checkout_resolve(repo: &Repository, gitref: Option<&str>) -> Result<(String, String), GitCacheError> {
    let resolved_ref = match gitref {
        Some(g) => g.to_string(),
        None => find_default_branch(repo)?,
    };
    // A branch is checked out via its remote-tracking ref; a tag/commit by its own name.
    let spec = if is_branch(repo, &resolved_ref) {
        format!("origin/{resolved_ref}")
    } else {
        resolved_ref.clone()
    };
    let object = repo.revparse_single(&spec)?;
    let commit = object.peel_to_commit()?;
    repo.checkout_tree(commit.as_object(), Some(git2::build::CheckoutBuilder::new().force()))?;
    repo.set_head_detached(commit.id())?;
    trace!("checked out {} @ {}", resolved_ref, commit.id());
    Ok((commit.id().to_string(), resolved_ref))
}

// ── small helpers ───────────────────────────────────────────────────────────────────────────────

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

fn interval_ms(opts: &FetchOptions) -> i64 {
    i64::try_from(opts.interval.as_millis()).unwrap_or(i64::MAX)
}

fn outcome(cache_path: &Utf8Path, freshness: Freshness, resolved_oid: String, resolved_ref: String) -> FetchOutcome {
    FetchOutcome {
        checkout_dir: cache_path.to_path_buf(),
        freshness,
        resolved_oid,
        resolved_ref,
    }
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
}
