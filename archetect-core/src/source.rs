use std::collections::HashSet;
use std::fs;
use std::process::{Command, Stdio};
use std::sync::{Mutex, OnceLock};

use camino::{Utf8Path, Utf8PathBuf};
use chrono::TimeZone;
use git2::Repository;
use log::{debug, info, trace, warn};
use regex::Regex;
use url::Url;

use crate::errors::SourceError;
use crate::utils::to_utf8_path_buf;
use crate::Archetect;

const ARCHETECT_PULLED: &'static str = "archetect.pulled";

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
        if self.archetect.configuration().locals().enabled() {
            match &self.source_type {
                SourceType::RemoteGit {
                    url: _,
                    cache_path: _,
                    directory_name,
                    gitref: _,
                } => {
                    if let Some(directory_name) = directory_name {
                        for local_root in self.archetect.configuration().locals().paths() {
                            match shellexpand::full(local_root.as_str()) {
                                Ok(expanded_root) => {
                                    let local_directory = Utf8PathBuf::from(expanded_root.as_ref());
                                    let local_directory = local_directory.join(directory_name);
                                    if local_directory.is_dir() {
                                        warn!("Using local: {}", local_directory);
                                        return Ok(local_directory);
                                    }
                                }
                                Err(err) => {
                                    warn!("Locals Path in archetect.yaml is invalid: {}", err);
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        Ok(self.source_type.local_path().to_path_buf())
    }

    pub fn source_type(&self) -> &SourceType {
        &self.source_type
    }

    pub fn source_contents(&self) -> SourceContents {
        if self.source_type().directory().join("catalog.yaml").is_file() ||
            self.source_type().directory().join("catalog.yml").is_file() {
            return SourceContents::Catalog;
        }

        if self.source_type().directory().join("archetype.yaml").is_file() ||
            self.source_type().directory().join("archetype.yml").is_file() {
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
                    cache_git_repo(&self.archetect, url, &gitref, &cache_path, true)?;
                }
            }
            SourceCommand::Invalidate => {
                if let SourceType::RemoteGit {
                    cache_path,
                    ..
                } = &self.source_type
                {
                    let repo = Repository::open(cache_path.join(".git"))?;
                    invalidate_timestamp(&repo)?;
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
    Catalog,
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
    REGEX.get_or_init(|| Regex::new(r"\S+@(\S+):(.*)").unwrap())
}

fn cached_paths() -> &'static Mutex<HashSet<String>> {
    static CACHED_PATHS: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    CACHED_PATHS.get_or_init(|| Mutex::new(HashSet::new()))
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

            let gitref = if url_parts.len() > 1 {
                Some(url_parts[1].to_owned())
            } else {
                None
            };
            cache_git_repo(&archetect, url_parts[0], &gitref, &cache_path, false)?;
            return Ok(SourceType::RemoteGit {
                url: url_parts[0].to_string(),
                cache_path,
                directory_name,
                gitref,
            });
        };

        if let Ok(url) = Url::parse(path) {
            if path.contains(".git") && url.has_host() {
                let cache_path =
                    cache_dir
                        .clone()
                        .join(get_cache_key(format!("{}/{}", url.host_str().unwrap(), url.path())));
                let directory_name = Utf8PathBuf::from(url.path()).file_stem().map(|stem| stem.to_string());

                let gitref = url.fragment().map(|r| r.to_owned());
                cache_git_repo(&archetect, url_parts[0], &gitref, &cache_path, false)?;
                return Ok(SourceType::RemoteGit {
                    url: path.to_owned(),
                    cache_path: cache_path,
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

        return if let Ok(path) = shellexpand::full(&path) {
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
        };
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

fn should_pull(repo: &Repository, archetect: &Archetect) -> Result<bool, SourceError> {
    if archetect.is_offline() {
        return Ok(false);
    }
    if archetect.configuration().updates().force() {
        return Ok(true);
    }

    let config = repo.config()?;
    if let Ok(timestamp) = config.get_i64(ARCHETECT_PULLED) {
        let timestamp = chrono::Utc.timestamp_millis_opt(timestamp);
        let now = chrono::Utc::now();
        let delta = now - timestamp.unwrap();
        Ok(delta > archetect.configuration().updates().interval())
    } else {
        Ok(true)
    }
}

fn write_timestamp(repo: &Repository) -> Result<(), SourceError> {
    let mut config = repo.config()?;
    config.set_i64(ARCHETECT_PULLED, chrono::Utc::now().timestamp_millis())?;
    Ok(())
}


fn invalidate_timestamp(repo: &Repository) -> Result<(), SourceError> {
    let mut config = repo.config()?;
    if let Ok(_value) = config.get_string(ARCHETECT_PULLED) {
        config.remove(ARCHETECT_PULLED)?;
    }
    Ok(())
}

fn cache_git_repo(
    archetect: &Archetect,
    url: &str,
    gitref: &Option<String>,
    cache_destination: &Utf8Path,
    force_pull: bool,
) -> Result<(), SourceError> {
    if !cache_destination.exists() {
        if !archetect.is_offline() {
            if cached_paths().lock().unwrap().insert(url.to_owned()) {
                info!("Cloning {}", url);
                debug!("Cloning to {}", cache_destination.as_str());
                handle_git(Command::new("git").args(["clone", url, cache_destination.as_str()]))?;
                let repo = git2::Repository::open(cache_destination.join(".git"))?;
                write_timestamp(&repo)?;
            }
        } else {
            return Err(SourceError::OfflineAndNotCached(url.to_owned()));
        }
    } else {
        let repo = Repository::open(cache_destination.join(".git"))?;
        if force_pull || should_pull(&repo, &archetect)? {
            if cached_paths().lock().unwrap().insert(url.to_owned()) {
                info!("Fetching {}", url);
                handle_git(Command::new("git").current_dir(cache_destination).args(["fetch"]))?;
                write_timestamp(&repo)?;
            }
        } else {
            trace!("Using cache for {}", url);
        }
    }

    let gitref = if let Some(gitref) = gitref {
        gitref.to_owned()
    } else {
        find_default_branch(cache_destination.as_str())?
    };

    let gitref_spec = if is_branch(cache_destination.as_str(), &gitref) {
        format!("origin/{}", &gitref)
    } else {
        gitref
    };

    debug!("Checking out {}", gitref_spec);
    handle_git(
        Command::new("git")
            .current_dir(cache_destination)
            .args(["checkout", &gitref_spec]),
    )?;

    Ok(())
}

fn is_branch(path: &str, gitref: &str) -> bool {
    handle_git(
        Command::new("git")
            .current_dir(path)
            .arg("show-ref")
            .arg("-q")
            .arg("--verify")
            .arg(format!("refs/remotes/origin/{}", gitref)),
    )
    .is_ok()
}

fn find_default_branch(path: &str) -> Result<String, SourceError> {
    for candidate in &["develop", "main", "master"] {
        if is_branch(path, candidate) {
            return Ok((*candidate).to_owned());
        }
    }
    Err(SourceError::NoDefaultBranch)
}

fn handle_git(command: &mut Command) -> Result<(), SourceError> {
    if cfg!(target_os = "windows") {
        command.stdin(Stdio::inherit());
        command.stderr(Stdio::inherit());
    }
    match command.output() {
        Ok(output) => match output.status.code() {
            Some(0) => Ok(()),
            Some(error_code) => Err(SourceError::RemoteSourceError(format!(
                "Error Code: {}\n{}",
                error_code,
                String::from_utf8(output.stderr)
                    .unwrap_or("Error reading error code from failed git command".to_owned())
            ))),
            None => Err(SourceError::RemoteSourceError("Git interrupted by signal".to_owned())),
        },
        Err(err) => Err(SourceError::IoError(err)),
    }
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
