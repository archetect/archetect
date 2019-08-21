use regex::Regex;
use std::path::{Path, PathBuf};
use std::process::Command;
use url::Url;

use log::{debug, info, trace};
use std::collections::HashSet;
use std::sync::Mutex;

#[derive(Debug, PartialOrd, PartialEq)]
pub enum Location {
    RemoteGit { url: String, path: PathBuf },
    LocalDirectory { path: PathBuf },
}

#[derive(Debug, PartialOrd, PartialEq)]
pub enum LocationError {
    LocationUnsupported,
    LocationNotFound,
    LocationInvalidPath,
    LocationInvalidEncoding,
    OfflineAndNotCached,
}

lazy_static! {
    static ref SHORT_GIT_PATTERN: Regex = Regex::new(r"\S+@(\S+):(.*)").unwrap();
    static ref CACHED_PATHS: Mutex<HashSet<String>> = Mutex::new(HashSet::new());
}

impl Location {
    pub fn detect<P: Into<String>>(path: P, offline: bool) -> Result<Location, LocationError> {
        let path = path.into();

        let app_root = directories::ProjectDirs::from("", "", "archetect").unwrap();
        let cache_root = app_root.cache_dir();

        if let Some(captures) = SHORT_GIT_PATTERN.captures(&path) {
            let cache_path = cache_root.clone().join(format!(
                "{}_{}",
                &captures[1],
                &captures[2].replace("/", ".")
            ));
            if let Some(error) = cache_git_repo(&path, &cache_path, offline) {
                return Err(error);
            }
            return Ok(Location::RemoteGit {
                url: path.to_owned(),
                path: cache_path,
            });
        };

        if let Ok(url) = Url::parse(&path) {
            if path.ends_with(".git") && url.has_host() {
                let cache_path = cache_root.clone().join(format!(
                    "{}_{}",
                    url.host_str().unwrap(),
                    url.path().trim_start_matches('/').replace("/", ".")
                ));
                if let Some(error) = cache_git_repo(&path, &cache_path, offline) {
                    return Err(error);
                }
                return Ok(Location::RemoteGit {
                    url: path,
                    path: cache_path,
                });
            }

            if let Ok(local_path) = url.to_file_path() {
                if local_path.exists() {
                    return Ok(Location::LocalDirectory { path: local_path });
                } else {
                    return Err(LocationError::LocationNotFound);
                }
            } else {
                return Err(LocationError::LocationUnsupported);
            }
        }

        if let Ok(path) = shellexpand::full(&path) {
            let local_path = PathBuf::from(path.as_ref());
            if local_path.exists() {
                if local_path.is_dir() {
                    return Ok(Location::LocalDirectory { path: local_path });
                } else {
                    return Err(LocationError::LocationUnsupported);
                }
            } else {
                return Err(LocationError::LocationNotFound);
            }
        } else {
            return Err(LocationError::LocationInvalidPath);
        }
    }
}

fn cache_git_repo(url: &str, cache_destination: &Path, offline: bool) -> Option<LocationError> {
    if !cache_destination.exists() {
        if !offline && CACHED_PATHS.lock().unwrap().insert(url.to_owned()) {
            info!("Cloning {}", url);
            let output = Command::new("git")
                .args(&["clone", &url, &format!("{}", cache_destination.display())])
                .output()
                .unwrap();
            debug!("Output: {:?}", String::from_utf8(output.stderr).unwrap());

            trace!("Cloned to {}", cache_destination.display());
            None
        } else {
            Some(LocationError::OfflineAndNotCached)
        }
    } else {
        if !offline && CACHED_PATHS.lock().unwrap().insert(url.to_owned()) {
            debug!("Resetting {}", url);
            Command::new("git")
                .current_dir(&cache_destination)
                .args(&["reset", "hard"])
                .output()
                .unwrap();
            info!("Pulling {}", url);
            Command::new("git")
                .current_dir(&cache_destination)
                .args(&["pull"])
                .output()
                .unwrap();
        }
        None
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
