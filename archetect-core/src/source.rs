use std::sync::OnceLock;

use archetect_git_cache::Lease;
use camino::{Utf8Path, Utf8PathBuf};
use log::{info, trace, warn};
use regex::Regex;
use url::Url;

use crate::errors::SourceError;
use crate::utils::to_utf8_path_buf;
use crate::Archetect;

pub struct Source {
    archetect: Archetect,
    source_type: SourceType,
    // Held for the Source's lifetime (== the render session, since `Source` is retained in
    // `Archetype::Inner`): a shared lease on the immutable tree, so the reaper can't reclaim it while
    // it's being rendered. `None` for local sources.
    _lease: Option<Lease>,
}

impl Source {
    pub fn new(archetect: Archetect, path: &str) -> Result<Self, SourceError> {
        let (source_type, lease) = SourceType::create(&archetect, path)?;
        Ok(Source {
            archetect,
            source_type,
            _lease: lease,
        })
    }

    pub fn path(&self) -> Result<Utf8PathBuf, SourceError> {
        // Primary locals resolution happens in SourceType::create. This block
        // is a safety net for SourceTypes created before locals was enabled
        // (or for cached RemoteGit entries that later got a local checkout).
        if let SourceType::RemoteGit { directory_name: Some(name), .. } = &self.source_type {
            if let Some(dir) = try_resolve_local(&self.archetect, name) {
                warn!("Using local: {}", dir);
                return Ok(dir);
            }
        }

        Ok(self.source_type.local_path().to_path_buf())
    }

    pub fn source_type(&self) -> &SourceType {
        &self.source_type
    }

    pub fn source_contents(&self) -> SourceContents {
        let dir = self.source_type().directory();

        // Archetype manifest: archetype.yaml/yml (canonical) or archetect.yaml/yml (alias).
        // May also contain catalog entries — handled at render time.
        if dir.join("archetype.yaml").is_file() || dir.join("archetype.yml").is_file() {
            return SourceContents::Archetype;
        }

        // Alias form: archetect.yaml/yml (accepted for compatibility)
        if dir.join("archetect.yaml").is_file() || dir.join("archetect.yml").is_file() {
            return SourceContents::Archetype;
        }

        SourceContents::Unknown
    }

    pub fn execute(&self, command: SourceCommand) -> Result<(), SourceError> {
        if let SourceType::RemoteGit { url, gitref, .. } = &self.source_type {
            let cache_root = self.archetect.layout().cache_dir();
            match command {
                SourceCommand::Pull => {
                    // Force a refresh; the returned lease is dropped immediately (pre-caching, not
                    // rendering — nothing reads the tree here).
                    let _ = resolve_git_source_with(
                        &self.archetect,
                        url,
                        gitref.as_deref(),
                        archetect_git_cache::PullPolicy::Always,
                        None,
                    )?;
                }
                SourceCommand::Refresh => {
                    // The eager loop's step: a GATED resolve whose TTL is the refresh cadence, so
                    // each cycle costs one ls-remote per moving ref and fetches only what moved.
                    // (A `Pull` here would re-fetch every repo every cycle; the strategy's own
                    // `IfMissing` would never probe at all.)
                    let interval = self
                        .archetect
                        .configuration()
                        .updates()
                        .refresh_interval()
                        .to_std()
                        .unwrap_or_else(|_| std::time::Duration::from_secs(300));
                    let _ = resolve_git_source_with(
                        &self.archetect,
                        url,
                        gitref.as_deref(),
                        archetect_git_cache::PullPolicy::Gated,
                        Some(interval),
                    )?;
                }
                SourceCommand::Invalidate | SourceCommand::Delete => {
                    archetect_git_cache::invalidate_all(&cache_root, url)?;
                }
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy)]
pub enum SourceContents {
    Archetype,
    Unknown,
}

#[derive(Clone, Copy)]
pub enum SourceCommand {
    /// Force-fetch, skipping the freshness gate (`cache pull`, pre-caching).
    Pull,
    /// One step of the eager background loop: a gated probe on the refresh cadence, fetching
    /// only what actually moved.
    Refresh,
    Invalidate,
    Delete,
}

//noinspection SpellCheckingInspection
#[derive(Clone, Debug, PartialOrd, PartialEq)]
pub enum SourceType {
    RemoteGit {
        url: String,
        /// The immutable, content-addressed tree the source resolved to (`trees/<hash>/<oid>/`).
        tree_dir: Utf8PathBuf,
        directory_name: Option<String>,
        gitref: Option<String>,
    },
    LocalDirectory {
        path: Utf8PathBuf,
    },
    LocalFile {
        path: Utf8PathBuf,
    },
}

fn ssh_git_pattern() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r"\S+@(\S+):(.*)").expect("hardcoded SSH git pattern is valid"))
}

// If locals is enabled and `directory_name` matches a directory under one of
// the configured `locals.paths`, return that local path. Used by both
// SourceType::create (short-circuit remote cloning when a local exists) and
// Source::path (fall back to local for sources that were cached previously
// but now have a local checkout).
fn try_resolve_local(archetect: &Archetect, directory_name: &str) -> Option<Utf8PathBuf> {
    if !archetect.configuration().locals().enabled() {
        return None;
    }
    for local_root in archetect.configuration().locals().paths() {
        let expanded_root = match shellexpand::full(local_root.as_str()) {
            Ok(p) => p,
            Err(err) => {
                warn!("Locals Path in archetect.yaml is invalid: {}", err);
                continue;
            }
        };
        let local_directory = Utf8PathBuf::from(expanded_root.as_ref()).join(directory_name);
        if local_directory.is_dir() {
            return Some(local_directory);
        }
    }
    None
}

impl SourceType {
    /// Parse a source string and resolve it. For a remote git source this resolves through the
    /// content-addressed cache and returns the session `Lease` (the caller holds it for the render);
    /// local sources return `None`.
    pub fn create(archetect: &Archetect, path: &str) -> Result<(SourceType, Option<Lease>), SourceError> {
        let url_parts: Vec<&str> = path.split('#').collect();
        if let Some(captures) = ssh_git_pattern().captures(url_parts[0]) {
            let repo_path = Utf8PathBuf::from(&captures[2]);
            let directory_name = repo_path.file_stem().map(|stem| stem.to_string());

            // Short-circuit the remote clone if the user has a local checkout of this repo under one
            // of the configured `locals.paths`. Keeps authoring loops working for archetypes whose
            // remote URL doesn't exist yet (e.g., a rename in progress).
            if let Some(dir) = directory_name.as_deref().and_then(|n| try_resolve_local(archetect, n)) {
                warn!("Using local: {}", dir);
                return Ok((SourceType::LocalDirectory { path: dir }, None));
            }

            let gitref = (url_parts.len() > 1).then(|| url_parts[1].to_owned());
            let url = url_parts[0].to_string();
            let (tree_dir, lease) = resolve_git_source(archetect, &url, gitref.as_deref(), false)?;
            return Ok((
                SourceType::RemoteGit { url, tree_dir, directory_name, gitref },
                Some(lease),
            ));
        };

        if let Ok(url) = Url::parse(path) {
            if url.host_str().filter(|_| path.contains(".git")).is_some() {
                let directory_name = Utf8PathBuf::from(url.path()).file_stem().map(|stem| stem.to_string());

                // Short-circuit the remote clone if the user has a local checkout — see above.
                if let Some(dir) = directory_name.as_deref().and_then(|n| try_resolve_local(archetect, n)) {
                    warn!("Using local: {}", dir);
                    return Ok((SourceType::LocalDirectory { path: dir }, None));
                }

                let gitref = url.fragment().map(|r| r.to_owned());
                // The fetch URL is the part before the `#fragment`; store that (so `execute`'s
                // pull/invalidate hash the same URL the cache keyed on).
                let source_url = url_parts[0].to_string();
                let (tree_dir, lease) = resolve_git_source(archetect, &source_url, gitref.as_deref(), false)?;
                return Ok((
                    SourceType::RemoteGit { url: source_url, tree_dir, directory_name, gitref },
                    Some(lease),
                ));
            }

            if let Ok(local_path) = url.to_file_path() {
                let local_path = to_utf8_path_buf(local_path);
                return if local_path.exists() {
                    Ok((SourceType::LocalDirectory { path: local_path }, None))
                } else {
                    Err(SourceError::SourceNotFound(local_path.to_string()))
                };
            }
        }

        if let Ok(path) = shellexpand::full(&path) {
            let local_path = Utf8PathBuf::from(path.as_ref());
            if local_path.exists() {
                if local_path.is_dir() {
                    Ok((SourceType::LocalDirectory { path: local_path }, None))
                } else {
                    Ok((SourceType::LocalFile { path: local_path }, None))
                }
            } else {
                Err(SourceError::SourceNotFound(local_path.to_string()))
            }
        } else {
            Err(SourceError::SourceInvalidPath(path.to_string()))
        }
    }

    pub fn directory(&self) -> &Utf8Path {
        match self {
            SourceType::RemoteGit { tree_dir, .. } => tree_dir.as_path(),
            SourceType::LocalDirectory { path } => path.as_path(),
            SourceType::LocalFile { path } => path.parent().unwrap_or(path),
        }
    }

    pub fn local_path(&self) -> &Utf8Path {
        match self {
            SourceType::RemoteGit { tree_dir, .. } => tree_dir.as_path(),
            SourceType::LocalDirectory { path } => path.as_path(),
            SourceType::LocalFile { path } => path.as_path(),
        }
    }

    pub fn source(&self) -> &str {
        match self {
            SourceType::RemoteGit { url, .. } => url,
            SourceType::LocalDirectory { path } => path.as_str(),
            SourceType::LocalFile { path } => path.as_str(),
        }
    }
}

/// Resolve a git source through the content-addressed cache, returning the immutable tree dir and the
/// session lease (hold it for as long as the tree is read). The crate owns the `sources/`+`trees/`
/// layout under the cache dir and the freshness gate; archetect just supplies its config.
///
/// The pull policy follows the process's [`UpdateStrategy`](crate::configuration::UpdateStrategy):
/// lazy resolves through the gate (the cold path pays for freshness); eager serves the cache as-is
/// (`IfMissing`) and leaves freshness to the background refresher. `-U`/`updates.force` wins over
/// both.
fn resolve_git_source(
    archetect: &Archetect,
    url: &str,
    gitref: Option<&str>,
    force_pull: bool,
) -> Result<(Utf8PathBuf, Lease), SourceError> {
    use archetect_git_cache::PullPolicy;

    use crate::configuration::UpdateStrategy;

    let updates = archetect.configuration().updates();
    let pull = if force_pull || updates.force() {
        PullPolicy::Always
    } else if updates.strategy() == UpdateStrategy::Eager {
        PullPolicy::IfMissing
    } else {
        PullPolicy::Gated
    };
    resolve_git_source_with(archetect, url, gitref, pull, None)
}

/// [`resolve_git_source`] with an explicit policy (and optionally a TTL other than
/// `updates.interval`) — the refresher's entry point, since its gated probes run on the refresh
/// cadence rather than the lazy TTL.
fn resolve_git_source_with(
    archetect: &Archetect,
    url: &str,
    gitref: Option<&str>,
    pull: archetect_git_cache::PullPolicy,
    interval: Option<std::time::Duration>,
) -> Result<(Utf8PathBuf, Lease), SourceError> {
    use archetect_git_cache::{FetchOptions, Freshness, RefPin};

    let interval = interval.unwrap_or_else(|| {
        archetect
            .configuration()
            .updates()
            .interval()
            .to_std()
            .unwrap_or_else(|_| std::time::Duration::from_secs(86400))
    });
    let opts = FetchOptions {
        pull,
        offline: archetect.is_offline(),
        interval,
        // archetect parses `url#ref` without knowing whether `ref` is a tag or a branch — let the
        // crate infer immutability from how the ref resolves locally.
        pin: RefPin::Infer,
    };

    let cache_root = archetect.layout().cache_dir();
    let resolved = archetect_git_cache::resolve(url, gitref, &cache_root, &opts)?;

    match resolved.freshness {
        Freshness::Cloned => info!("Cloning {}", url),
        Freshness::Updated => info!("Updating {}", url),
        Freshness::UpToDate { .. } => trace!("Using cache for {}", url),
    }

    Ok((resolved.tree_dir, resolved.lease))
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ssh_git_pattern() {
        let captures = ssh_git_pattern()
            .captures("git@github.com:archetect/archetect.git")
            .unwrap();
        assert_eq!(&captures[1], "github.com");
        assert_eq!(&captures[2], "archetect/archetect.git");
    }
}
