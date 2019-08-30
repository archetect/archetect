use regex::Regex;
use std::path::{Path, PathBuf};
use std::process::Command;
use url::Url;

use log::{debug, info, trace};
use std::collections::HashSet;
use std::sync::Mutex;

use crate::Archetect;

#[derive(Clone, Debug, PartialOrd, PartialEq)]
pub enum Source {
    RemoteGit { url: String, path: PathBuf },
    LocalDirectory { path: PathBuf },
}

#[derive(Debug, PartialOrd, PartialEq)]
pub enum SourceError {
    SourceUnsupported(String),
    SourceNotFound(String),
    SourceInvalidPath(String),
    SourceInvalidEncoding(String),
    RemoteSourceError(String),
    OfflineAndNotCached(String),
    IOError(String),
}

lazy_static! {
    static ref SHORT_GIT_PATTERN: Regex = Regex::new(r"\S+@(\S+):(.*)").unwrap();
    static ref CACHED_PATHS: Mutex<HashSet<String>> = Mutex::new(HashSet::new());
}

impl Source {
    pub fn detect(archetect: &Archetect, path: &str, relative_to: Option<Source>) -> Result<Source, SourceError> {
        let git_cache = archetect.layout().git_cache_dir();

        if let Some(captures) = SHORT_GIT_PATTERN.captures(&path) {
            let cache_path = git_cache
                .clone()
                .join(format!("{}_{}", &captures[1], &captures[2].replace("/", ".")));
            if let Err(error) = cache_git_repo(&path, &cache_path, archetect.offline()) {
                return Err(error);
            }
            return Ok(Source::RemoteGit {
                url: path.to_owned(),
                path: cache_path,
            });
        };

        if let Ok(url) = Url::parse(&path) {
            if path.ends_with(".git") && url.has_host() {
                let cache_path = git_cache.clone().join(format!(
                    "{}_{}",
                    url.host_str().unwrap(),
                    url.path().trim_start_matches('/').replace("/", ".")
                ));
                if let Err(error) = cache_git_repo(&path, &cache_path, archetect.offline()) {
                    return Err(error);
                }
                return Ok(Source::RemoteGit {
                    url: path.to_owned(),
                    path: cache_path,
                });
            }

            if let Ok(local_path) = url.to_file_path() {
                if local_path.exists() {
                    return Ok(Source::LocalDirectory { path: local_path });
                } else {
                    return Err(SourceError::SourceNotFound(local_path.display().to_string()));
                }
            } else {
                return Err(SourceError::SourceUnsupported(path.to_string()));
            }
        }

        if let Ok(path) = shellexpand::full(&path) {
            let local_path = PathBuf::from(path.as_ref());
            if local_path.is_relative() {
                if let Some(parent) = relative_to {
                    let local_path = parent.local_path().clone().join(local_path);
                    if local_path.exists() && local_path.is_dir() {
                        return Ok(Source::LocalDirectory { path: local_path });
                    } else {
                        return Err(SourceError::SourceNotFound(local_path.display().to_string()));
                    }
                }
            }
            if local_path.exists() {
                if local_path.is_dir() {
                    return Ok(Source::LocalDirectory { path: local_path });
                } else {
                    return Err(SourceError::SourceUnsupported(local_path.display().to_string()));
                }
            } else {
                return Err(SourceError::SourceNotFound(local_path.display().to_string()));
            }
        } else {
            return Err(SourceError::SourceInvalidPath(path.to_string()));
        }
    }

    pub fn local_path(&self) -> &Path {
        match self {
            Source::LocalDirectory { path } => path.as_path(),
            Source::RemoteGit { url: _, path } => path.as_path(),
        }
    }
}

fn cache_git_repo(url: &str, cache_destination: &Path, offline: bool) -> Result<(), SourceError> {
    if !cache_destination.exists() {
        if !offline && CACHED_PATHS.lock().unwrap().insert(url.to_owned()) {
            info!("Cloning {}", url);
            handle_git(Command::new("git").args(&["clone", &url, &format!("{}", cache_destination.display())]))?;
            trace!("Cloned to {}", cache_destination.display());
            Ok(())
        } else {
            Err(SourceError::OfflineAndNotCached(url.to_owned()))
        }
    } else {
        if !offline && CACHED_PATHS.lock().unwrap().insert(url.to_owned()) {
            debug!("Resetting {}", url);
            handle_git(
                Command::new("git")
                    .current_dir(&cache_destination)
                    .args(&["reset", "--hard"]),
            )?;

            info!("Pulling {}", url);
            handle_git(Command::new("git").current_dir(&cache_destination).args(&["pull"]))?;
        }
        Ok(())
    }
}

fn handle_git(command: &mut Command) -> Result<(), SourceError> {
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
        Err(err) => Err(SourceError::IOError(err.to_string())),
    }
}

#[cfg(test)]
mod tests {
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
    //        let captures = SHORT_GIT_PATTERN
    //            .captures("git@github.com:jimmiebfulton/archetect.git")
    //            .unwrap();
    //        assert_eq!(&captures[1], "github.com");
    //        assert_eq!(&captures[2], "jimmiebfulton/archetect.git");
    //    }
}
