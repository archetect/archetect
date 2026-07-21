use std::fs;
use std::sync::OnceLock;

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
}

impl Source {
    pub fn new(archetect: Archetect, path: &str) -> Result<Self, SourceError> {
        let source_type = SourceType::create(&archetect, path)?;
        Ok(Source { archetect, source_type })
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
        match command {
            SourceCommand::Pull => {
                if let SourceType::RemoteGit {
                    url,
                    cache_path,
                    gitref,
                    ..
                } = &self.source_type
                {
                    cache_git_repo(&self.archetect, url, gitref, cache_path, true)?;
                }
            }
            SourceCommand::Invalidate => {
                if let SourceType::RemoteGit { cache_path, .. } = &self.source_type {
                    archetect_git_cache::invalidate_all(cache_path)?;
                }
            }
            SourceCommand::Delete => {
                if let SourceType::RemoteGit { cache_path, .. } = &self.source_type {
                    fs::remove_dir_all(cache_path)?;
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
    Pull,
    Invalidate,
    Delete,
}

//noinspection SpellCheckingInspection
#[derive(Clone, Debug, PartialOrd, PartialEq)]
pub enum SourceType {
    RemoteGit {
        url: String,
        cache_path: Utf8PathBuf,
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
    pub fn create(archetect: &Archetect, path: &str) -> Result<SourceType, SourceError> {
        let cache_dir = archetect.layout().cache_dir();

        let url_parts: Vec<&str> = path.split('#').collect();
        if let Some(captures) = ssh_git_pattern().captures(url_parts[0]) {
            let cache_path = cache_dir
                .clone()
                .join(get_cache_key(format!("{}/{}", &captures[1], &captures[2])));

            let repo_path = Utf8PathBuf::from(&captures[2]);
            let directory_name = repo_path.file_stem().map(|stem| stem.to_string());

            // Short-circuit the remote clone if the user has a local checkout
            // of this repo under one of the configured `locals.paths`. Keeps
            // authoring loops working for archetypes whose remote URL doesn't
            // exist yet (e.g., a rename in progress).
            if let Some(dir) = directory_name.as_deref().and_then(|n| try_resolve_local(archetect, n)) {
                warn!("Using local: {}", dir);
                return Ok(SourceType::LocalDirectory { path: dir });
            }

            let gitref = if url_parts.len() > 1 {
                Some(url_parts[1].to_owned())
            } else {
                None
            };
            cache_git_repo(archetect, url_parts[0], &gitref, &cache_path, false)?;
            return Ok(SourceType::RemoteGit {
                url: url_parts[0].to_string(),
                cache_path,
                directory_name,
                gitref,
            });
        };

        if let Ok(url) = Url::parse(path) {
            if let Some(host) = url.host_str().filter(|_| path.contains(".git")) {
                let cache_path =
                    cache_dir
                        .clone()
                        .join(get_cache_key(format!("{}/{}", host, url.path())));
                let directory_name = Utf8PathBuf::from(url.path()).file_stem().map(|stem| stem.to_string());

                // Short-circuit the remote clone if the user has a local
                // checkout — see the SSH branch above for the same pattern.
                if let Some(dir) = directory_name.as_deref().and_then(|n| try_resolve_local(archetect, n)) {
                    warn!("Using local: {}", dir);
                    return Ok(SourceType::LocalDirectory { path: dir });
                }

                let gitref = url.fragment().map(|r| r.to_owned());
                cache_git_repo(archetect, url_parts[0], &gitref, &cache_path, false)?;
                return Ok(SourceType::RemoteGit {
                    url: path.to_owned(),
                    cache_path,
                    directory_name,
                    gitref,
                });
            }

            if let Ok(local_path) = url.to_file_path() {
                let local_path = to_utf8_path_buf(local_path);
                return if local_path.exists() {
                    Ok(SourceType::LocalDirectory { path: local_path })
                } else {
                    Err(SourceError::SourceNotFound(local_path.to_string()))
                };
            }
        }

        if let Ok(path) = shellexpand::full(&path) {
            let local_path = Utf8PathBuf::from(path.as_ref());
            if local_path.exists() {
                if local_path.is_dir() {
                    Ok(SourceType::LocalDirectory { path: local_path })
                } else {
                    Ok(SourceType::LocalFile { path: local_path })
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
            SourceType::RemoteGit {
                url: _,
                cache_path: path,
                directory_name: _,
                gitref: _,
            } => path.as_path(),
            SourceType::LocalDirectory { path } => path.as_path(),
            SourceType::LocalFile { path } => path.parent().unwrap_or(path),
        }
    }

    pub fn local_path(&self) -> &Utf8Path {
        match self {
            SourceType::RemoteGit {
                url: _,
                cache_path: path,
                directory_name: _,
                gitref: _,
            } => path.as_path(),
            SourceType::LocalDirectory { path } => path.as_path(),
            SourceType::LocalFile { path } => path.as_path(),
        }
    }

    pub fn source(&self) -> &str {
        match self {
            SourceType::RemoteGit {
                url,
                cache_path: _,
                directory_name: _,
                gitref: _,
            } => url,
            SourceType::LocalDirectory { path } => path.as_str(),
            SourceType::LocalFile { path } => path.as_str(),
        }
    }
}

fn get_cache_hash<S: AsRef<[u8]>>(input: S) -> u64 {
    let result = farmhash::fingerprint64(input.as_ref());
    result
}

fn get_cache_key<S: AsRef<[u8]>>(input: S) -> String {
    format!("{}", get_cache_hash(input))
}

/// Clone/refresh a git source into `cache_destination` and check out `gitref`, delegating the
/// two-gate (TTL + remote-hash) freshness logic to the shared `archetect-git-cache` crate. This is a
/// thin adapter: archetect owns the cache directory scheme (the farmhash key above) and the
/// per-process fetch dedup; the crate owns clone/fetch/ls-remote/checkout and the freshness gates.
fn cache_git_repo(
    archetect: &Archetect,
    url: &str,
    gitref: &Option<String>,
    cache_destination: &Utf8Path,
    force_pull: bool,
) -> Result<(), SourceError> {
    use archetect_git_cache::{FetchOptions, Freshness, RefPin};

    let interval = archetect
        .configuration()
        .updates()
        .interval()
        .to_std()
        .unwrap_or_else(|_| std::time::Duration::from_secs(604800));
    let opts = FetchOptions {
        force: force_pull || archetect.configuration().updates().force(),
        offline: archetect.is_offline(),
        interval,
        // archetect parses `url#ref` without knowing whether `ref` is a tag or a branch — let the
        // crate infer immutability from how the ref resolves locally.
        pin: RefPin::Infer,
    };

    let gitref = gitref.as_deref();
    // Per-process dedup: the first sighting of a URL does the network-eligible fetch (which pulls
    // every ref via `--tags`); later sightings in the same run only need a local re-checkout.
    let outcome = if archetect.mark_source_fetched(url) {
        archetect_git_cache::fetch(url, gitref, cache_destination, &opts)?
    } else {
        archetect_git_cache::checkout(gitref, cache_destination)?
    };

    match outcome.freshness {
        Freshness::Cloned => info!("Cloning {}", url),
        Freshness::Updated => info!("Updating {}", url),
        Freshness::UpToDate { .. } => trace!("Using cache for {}", url),
    }

    Ok(())
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
