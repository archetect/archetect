use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;

use log::{debug, info, trace};
use regex::Regex;
use url::Url;

use crate::requirements::{Requirements, RequirementsError};
use crate::Archetect;

#[derive(Clone, Debug, PartialOrd, PartialEq)]
pub enum Source {
    RemoteGit { url: String, path: PathBuf },
    RemoteHttp { url: String, path: PathBuf },
    LocalDirectory { path: PathBuf },
    LocalFile { path: PathBuf },
}

#[derive(Debug)]
pub enum SourceError {
    SourceUnsupported(String),
    SourceNotFound(String),
    SourceInvalidPath(String),
    SourceInvalidEncoding(String),
    RemoteSourceError(String),
    OfflineAndNotCached(String),
    IoError(std::io::Error),
    RequirementsError { path: String, cause: RequirementsError },
}

impl From<std::io::Error> for SourceError {
    fn from(error: std::io::Error) -> SourceError {
        SourceError::IoError(error)
    }
}

lazy_static! {
    static ref SHORT_GIT_PATTERN: Regex = Regex::new(r"\S+@(\S+):(.*)").unwrap();
    static ref CACHED_PATHS: Mutex<HashSet<String>> = Mutex::new(HashSet::new());
}

impl Source {
    pub fn detect(archetect: &Archetect, path: &str, relative_to: Option<Source>) -> Result<Source, SourceError> {
        let source = path;
        let git_cache = archetect.layout().git_cache_dir();

        if let Some(captures) = SHORT_GIT_PATTERN.captures(&path) {
            let cache_path = git_cache
                .clone()
                .join(get_cache_key(format!("{}/{}", &captures[1], &captures[2])));
            if let Err(error) = cache_git_repo(&path, &cache_path, archetect.offline()) {
                return Err(error);
            }
            verify_requirements(archetect, source, &cache_path)?;
            return Ok(Source::RemoteGit {
                url: path.to_owned(),
                path: cache_path,
            });
        };

        if let Ok(url) = Url::parse(&path) {
            if path.ends_with(".git") && url.has_host() {
                let cache_path =
                    git_cache
                        .clone()
                        .join(get_cache_key(format!("{}/{}", url.host_str().unwrap(), url.path())));
                if let Err(error) = cache_git_repo(&path, &cache_path, archetect.offline()) {
                    return Err(error);
                }
                verify_requirements(archetect, source, &cache_path)?;
                return Ok(Source::RemoteGit {
                    url: path.to_owned(),
                    path: cache_path,
                });
            }

            if let Ok(local_path) = url.to_file_path() {
                if local_path.exists() {
                    verify_requirements(archetect, source, &local_path)?;
                    return Ok(Source::LocalDirectory { path: local_path });
                } else {
                    return Err(SourceError::SourceNotFound(local_path.display().to_string()));
                }
            }
        }

        if let Ok(path) = shellexpand::full(&path) {
            let local_path = PathBuf::from(path.as_ref());
            if local_path.is_relative() {
                if let Some(parent) = relative_to {
                    let local_path = parent.local_path().clone().join(local_path);
                    if local_path.exists() && local_path.is_dir() {
                        verify_requirements(archetect, source, &local_path)?;
                        return Ok(Source::LocalDirectory { path: local_path });
                    } else {
                        return Err(SourceError::SourceNotFound(local_path.display().to_string()));
                    }
                }
            }
            if local_path.exists() {
                if local_path.is_dir() {
                    verify_requirements(archetect, source, &local_path)?;
                    return Ok(Source::LocalDirectory { path: local_path });
                } else {
                    return Ok(Source::LocalFile { path: local_path });
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
            Source::RemoteGit { url: _, path } => path.as_path(),
            Source::RemoteHttp { url: _, path } => path.as_path(),
            Source::LocalDirectory { path } => path.as_path(),
            Source::LocalFile { path } => path.as_path(),
        }
    }

    pub fn source(&self) -> &str {
        match self {
            Source::RemoteGit { url, path: _ } => url,
            Source::RemoteHttp { url, path: _ } => url,
            Source::LocalDirectory { path } => path.to_str().unwrap(),
            Source::LocalFile { path } => path.to_str().unwrap(),
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

fn verify_requirements(archetect: &Archetect, source: &str, path: &Path) -> Result<(), SourceError> {
    match Requirements::load(&path) {
        Ok(results) => {
            if let Some(requirements) = results {
                if let Err(error) = requirements.verify(archetect) {
                    return Err(SourceError::RequirementsError {
                        path: source.to_owned(),
                        cause: error,
                    });
                }
            }
        }
        Err(error) => {
            return Err(SourceError::RequirementsError {
                path: path.display().to_string(),
                cause: error,
            });
        }
    }
    Ok(())
}

fn cache_git_repo(url: &str, cache_destination: &Path, offline: bool) -> Result<(), SourceError> {
    if !cache_destination.exists() {
        if !offline && CACHED_PATHS.lock().unwrap().insert(url.to_owned()) {
            info!("Cloning {}", url);
            trace!("Cloning to {}", cache_destination.to_str().unwrap());
            handle_git(Command::new("git").args(&["clone", &url, cache_destination.to_str().unwrap()]))?;
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
            get_cache_hash("https://raw.githubusercontent.com/archetect/archetect/master/LICENSE-MIT")
        );
        println!(
            "{}",
            get_cache_hash("https://raw.githubusercontent.com/archetect/archetect/master/LICENSE-MIT")
        );
        println!("{}", get_cache_hash("f"));
        println!("{}", get_cache_hash("1"));
    }

    #[test]
    fn test_http_source() {
        let archetect = archetect_core::build().unwrap();
        let source = Source::detect(
            &archetect,
            "https://raw.githubusercontent.com/archetect/archetect/master/LICENSE-MIT",
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
    //        let captures = SHORT_GIT_PATTERN
    //            .captures("git@github.com:jimmiebfulton/archetect.git")
    //            .unwrap();
    //        assert_eq!(&captures[1], "github.com");
    //        assert_eq!(&captures[2], "jimmiebfulton/archetect.git");
    //    }
}
