use camino::{Utf8Path, Utf8PathBuf};
use std::collections::HashSet;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};

use log::{debug, info};
use regex::Regex;
use url::Url;

use crate::errors::SourceError;
use crate::utils::to_utf8_path_buf;
use crate::v2::runtime::context::RuntimeContext;
use crate::Archetect;

#[derive(Clone, Debug, PartialOrd, PartialEq)]
pub enum Source {
    RemoteGit {
        url: String,
        path: Utf8PathBuf,
        gitref: Option<String>,
    },
    RemoteHttp {
        url: String,
        path: Utf8PathBuf,
    },
    LocalDirectory {
        path: Utf8PathBuf,
    },
    LocalFile {
        path: Utf8PathBuf,
    },
}

lazy_static! {
    static ref SSH_GIT_PATTERN: Regex = Regex::new(r"\S+@(\S+):(.*)").unwrap();
    static ref CACHED_PATHS: Mutex<HashSet<String>> = Mutex::new(HashSet::new());
}

impl Source {
    pub fn detect(
        archetect: &Archetect,
        runtime_context: &RuntimeContext,
        path: &str,
        relative_to: Option<Source>,
    ) -> Result<Source, SourceError> {
        let git_cache = archetect.layout().git_cache_dir();

        let urlparts: Vec<&str> = path.split('#').collect();
        if let Some(captures) = SSH_GIT_PATTERN.captures(urlparts[0]) {
            let cache_path = git_cache
                .clone()
                .join(get_cache_key(format!("{}/{}", &captures[1], &captures[2])));

            let gitref = if urlparts.len() > 1 {
                Some(urlparts[1].to_owned())
            } else {
                None
            };
            cache_git_repo(urlparts[0], &gitref, &cache_path, runtime_context.offline())?;
            return Ok(Source::RemoteGit {
                url: path.to_owned(),
                path: cache_path,
                gitref,
            });
        };

        if let Ok(url) = Url::parse(path) {
            if path.contains(".git") && url.has_host() {
                let cache_path =
                    git_cache
                        .clone()
                        .join(get_cache_key(format!("{}/{}", url.host_str().unwrap(), url.path())));
                let gitref = url.fragment().map(|r| r.to_owned());
                cache_git_repo(urlparts[0], &gitref, &cache_path, runtime_context.offline())?;
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
            if local_path.is_relative() {
                if let Some(parent) = relative_to {
                    let local_path = parent.local_path().join(local_path);
                    if local_path.exists() && local_path.is_dir() {
                        return Ok(Source::LocalDirectory { path: local_path });
                    } else {
                        return Err(SourceError::SourceNotFound(local_path.to_string()));
                    }
                }
            }
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
            Source::RemoteHttp { url: _, path } => path.as_path(),
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
            Source::RemoteHttp { url: _, path } => path.as_path(),
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
            Source::RemoteHttp { url, path: _ } => url,
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

fn cache_git_repo(
    url: &str,
    gitref: &Option<String>,
    cache_destination: &Utf8Path,
    offline: bool,
) -> Result<(), SourceError> {
    if !cache_destination.exists() {
        if !offline && CACHED_PATHS.lock().unwrap().insert(url.to_owned()) {
            info!("Cloning {}", url);
            debug!("Cloning to {}", cache_destination.as_str());
            handle_git(Command::new("git").args(["clone", url, cache_destination.as_str()]))?;
        } else {
            return Err(SourceError::OfflineAndNotCached(url.to_owned()));
        }
    } else if !offline && CACHED_PATHS.lock().unwrap().insert(url.to_owned()) {
        info!("Fetching {}", url);
        handle_git(Command::new("git").current_dir(cache_destination).args(["fetch"]))?;
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
    use crate::configuration::Configuration;
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

    #[test]
    fn test_http_source() {
        let archetect = Archetect::build().unwrap();
        let configuration = Configuration::default();
        let runtime_context = RuntimeContext::new(&configuration, HashSet::new(), Utf8PathBuf::new());
        let source = Source::detect(
            &archetect,
            &runtime_context,
            "https://raw.githubusercontent.com/archetect/archetect/master/LICENSE-MIT-MIT",
            None,
        );
        println!("{:?}", source);
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
    //    #[test]
    //    fn test_short_git_pattern() {
    //        let captures = SSH_GIT_PATTERN
    //            .captures("git@github.com:jimmiebfulton/archetect.git")
    //            .unwrap();
    //        assert_eq!(&captures[1], "github.com");
    //        assert_eq!(&captures[2], "jimmiebfulton/archetect.git");
    //    }
}
