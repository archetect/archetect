use std::collections::HashSet;
use std::process::{Command, Stdio};
use std::sync::{Mutex, OnceLock};

use camino::{Utf8Path, Utf8PathBuf};
use chrono::TimeZone;
use git2::Repository;
use log::{debug, info, trace};
use regex::Regex;
use url::Url;

use crate::errors::SourceError;
use crate::runtime::context::RuntimeContext;
use crate::utils::to_utf8_path_buf;

//noinspection SpellCheckingInspection
#[derive(Clone, Debug, PartialOrd, PartialEq)]
pub enum Source {
    RemoteGit {
        url: String,
        path: Utf8PathBuf,
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

impl Source {
    pub fn create(runtime_context: &RuntimeContext, path: &str, force_pull: bool) -> Result<Source, SourceError> {
        let cache_dir = runtime_context.layout().cache_dir();

        let url_parts: Vec<&str> = path.split('#').collect();
        if let Some(captures) = ssh_git_pattern().captures(url_parts[0]) {
            let cache_path = cache_dir
                .clone()
                .join(get_cache_key(format!("{}/{}", &captures[1], &captures[2])));

            let gitref = if url_parts.len() > 1 {
                Some(url_parts[1].to_owned())
            } else {
                None
            };
            cache_git_repo(&runtime_context, url_parts[0], &gitref, &cache_path, force_pull)?;
            return Ok(Source::RemoteGit {
                url: path.to_owned(),
                path: cache_path,
                gitref,
            });
        };

        if let Ok(url) = Url::parse(path) {
            if path.contains(".git") && url.has_host() {
                let cache_path =
                    cache_dir
                        .clone()
                        .join(get_cache_key(format!("{}/{}", url.host_str().unwrap(), url.path())));
                let gitref = url.fragment().map(|r| r.to_owned());
                cache_git_repo(&runtime_context, url_parts[0], &gitref, &cache_path, force_pull)?;
                return Ok(Source::RemoteGit {
                    url: path.to_owned(),
                    path: cache_path,
                    gitref,
                });
            }

            if let Ok(local_path) = url.to_file_path() {
                let local_path = to_utf8_path_buf(local_path);
                return if local_path.exists() {
                    Ok(Source::LocalDirectory { path: local_path })
                } else {
                    Err(SourceError::SourceNotFound(local_path.to_string()))
                };
            }
        }

        return if let Ok(path) = shellexpand::full(&path) {
            let local_path = Utf8PathBuf::from(path.as_ref());
            if local_path.exists() {
                if local_path.is_dir() {
                    Ok(Source::LocalDirectory { path: local_path })
                } else {
                    Ok(Source::LocalFile { path: local_path })
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
            Source::RemoteGit {
                url: _,
                path,
                gitref: _,
            } => path.as_path(),
            Source::LocalDirectory { path } => path.as_path(),
            Source::LocalFile { path } => path.parent().unwrap_or(path),
        }
    }

    pub fn local_path(&self) -> &Utf8Path {
        match self {
            Source::RemoteGit {
                url: _,
                path,
                gitref: _,
            } => path.as_path(),
            Source::LocalDirectory { path } => path.as_path(),
            Source::LocalFile { path } => path.as_path(),
        }
    }

    pub fn source(&self) -> &str {
        match self {
            Source::RemoteGit {
                url,
                path: _,
                gitref: _,
            } => url,
            Source::LocalDirectory { path } => path.as_str(),
            Source::LocalFile { path } => path.as_str(),
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

fn should_pull(repo: &Repository, runtime_context: &RuntimeContext) -> Result<bool, SourceError> {
    if runtime_context.offline() {
        return Ok(false);
    }
    if runtime_context.updates().force() {
        return Ok(true);
    }

    let config = repo.config()?;
    if let Ok(timestamp) = config.get_i64("archetect.pulled") {
        let timestamp = chrono::Utc.timestamp_millis_opt(timestamp);
        let now = chrono::Utc::now();
        let delta = now - timestamp.unwrap();
        Ok(delta > runtime_context.updates().interval())
    } else {
        Ok(true)
    }
}

fn write_timestamp(repo: &Repository) -> Result<(), SourceError> {
    let mut config = repo.config()?;
    config.set_i64("archetect.pulled", chrono::Utc::now().timestamp_millis())?;
    Ok(())
}

fn cache_git_repo(
    runtime_context: &RuntimeContext,
    url: &str,
    gitref: &Option<String>,
    cache_destination: &Utf8Path,
    force_pull: bool,
) -> Result<(), SourceError> {
    if !cache_destination.exists() {
        if !runtime_context.offline() && cached_paths().lock().unwrap().insert(url.to_owned()) {
            info!("Cloning {}", url);
            debug!("Cloning to {}", cache_destination.as_str());
            handle_git(Command::new("git").args(["clone", url, cache_destination.as_str()]))?;
            let repo = git2::Repository::open(cache_destination.join(".git"))?;
            write_timestamp(&repo)?;
        } else {
            return Err(SourceError::OfflineAndNotCached(url.to_owned()));
        }
    } else if cached_paths().lock().unwrap().insert(url.to_owned()) {
        let repo = git2::Repository::open(cache_destination.join(".git"))?;
        if force_pull || should_pull(&repo, &runtime_context)? {
            info!("Fetching {}", url);
            handle_git(Command::new("git").current_dir(cache_destination).args(["fetch"]))?;
            write_timestamp(&repo)?;
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
    fn test_cache_hash() {
        println!(
            "{}",
            get_cache_hash("https://raw.githubusercontent.com/archetect/archetect/master/LICENSE-MIT-MIT")
        );
        println!(
            "{}",
            get_cache_hash("https://raw.githubusercontent.com/archetect/archetect/master/LICENSE-MIT-MIT")
        );
        println!("{}", get_cache_hash("f"));
        println!("{}", get_cache_hash("1"));
    }

    //    use super::*;
    //    use matches::assert_matches;

    //    #[test]
    //    fn test_detect_short_git_url() {
    //        // TODO: Fix this test.
    //        assert_matches!(
    //            Location::detect("git@github.com:jimmiebfulton/archetect.git", ),
    //            Ok(Location::RemoteGit { url: _, path: _ })
    //        );
    //    }
    //
    //    #[test]
    //    fn test_detect_http_git_url() {
    //        // TODO: Fix this test.
    //        assert_matches!(
    //            Location::detect("https://github.com/jimmiebfulton/archetect.git"),
    //            Ok(Location::RemoteGit { url: _, path: _ })
    //        );
    //    }
    //
    //    #[test]
    //    fn test_detect_local_directory() {
    //        assert_eq!(
    //            Location::detect(".", false),
    //            Ok(Location::LocalDirectory {
    //                path: PathBuf::from(".")
    //            })
    //        );
    //
    //        assert_matches!(
    //            Location::detect("~"),
    //            Ok(Location::LocalDirectory { path: _ })
    //        );
    //
    //        assert_eq!(
    //            Location::detect("notfound", false),
    //            Err(LocationError::LocationNotFound)
    //        );
    //    }
    //
    //    #[test]
    //    fn test_file_url() {
    //        assert_eq!(
    //            Location::detect("file://localhost/home", false),
    //            Ok(Location::LocalDirectory {
    //                path: PathBuf::from("/home")
    //            }),
    //        );
    //
    //        assert_eq!(
    //            Location::detect("file:///home", false),
    //            Ok(Location::LocalDirectory {
    //                path: PathBuf::from("/home")
    //            }),
    //        );
    //
    //        assert_eq!(
    //            Location::detect("file://localhost/nope", false),
    //            Err(LocationError::LocationNotFound),
    //        );
    //
    //        assert_eq!(
    //            Location::detect("file://nope/home", false),
    //            Err(LocationError::LocationUnsupported),
    //        );
    //    }
    //

    #[test]
    fn test_short_git_pattern() {
        let captures = ssh_git_pattern()
            .captures("git@github.com:jimmiebfulton/archetect.git")
            .unwrap();
        assert_eq!(&captures[1], "github.com");
        assert_eq!(&captures[2], "jimmiebfulton/archetect.git");
    }
}
