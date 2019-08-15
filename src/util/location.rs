use regex::Regex;
use std::env;
use std::path::PathBuf;
use url::Url;

#[derive(Debug, PartialOrd, PartialEq)]
pub enum Location {
    RemoteGit { url: String, path: PathBuf },
    LocalDirectory { path: PathBuf },
}

#[derive(Debug, PartialOrd, PartialEq)]
pub enum LocationError {
    Unsupported,
    NotFound,
    InvalidLocation,
    InvalidEncoding,
}

lazy_static! {
    static ref SHORT_GIT_PATTERN: Regex = Regex::new(r"\S+@(\S+):(.*)").unwrap();
}

impl Location {
    pub fn detect<P: Into<String>>(path: P) -> Result<Location, LocationError> {
        let path = path.into();

        if let Some(captures) = SHORT_GIT_PATTERN.captures(&path) {
            return Ok(Location::RemoteGit {
                url: path.to_owned(),
                path: env::temp_dir().join("archetect").join(format!(
                    "{}_{}",
                    &captures[1],
                    &captures[2].replace("/", ".")
                )),
            });
        };

        if let Ok(url) = Url::parse(&path) {
            if path.ends_with(".git") && url.has_host() {
                return Ok(Location::RemoteGit {
                    url: path,
                    path: env::temp_dir().join("archetect").join(format!(
                        "{}_{}",
                        url.host_str().unwrap(),
                        url.path().trim_start_matches('/').replace("/", ".")
                    )),
                });
            }

            if let Ok(local_path) = url.to_file_path() {
                if local_path.exists() {
                    return Ok(Location::LocalDirectory { path: local_path });
                } else {
                    return Err(LocationError::NotFound);
                }
            } else {
                return Err(LocationError::Unsupported);
            }
        }

        if let Ok(path) = shellexpand::full(&path) {
            let local_path = PathBuf::from(path.as_ref());
            if local_path.exists() {
                if local_path.is_dir() {
                    return Ok(Location::LocalDirectory { path: local_path });
                } else {
                    return Err(LocationError::Unsupported);
                }
            } else {
                return Err(LocationError::NotFound);
            }
        } else {
            return Err(LocationError::InvalidLocation);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use matches::assert_matches;

    #[test]
    fn test_detect_short_git_url() {
        // TODO: Fix this test.
        assert_matches!(
            Location::detect("git@github.com:jimmiebfulton/archetect.git"),
            Ok(Location::RemoteGit { url: _, path: _ })
        );
    }

    #[test]
    fn test_detect_http_git_url() {
        // TODO: Fix this test.
        assert_matches!(
            Location::detect("https://github.com/jimmiebfulton/archetect.git"),
            Ok(Location::RemoteGit { url: _, path: _ })
        );
    }

    #[test]
    fn test_detect_local_directory() {
        assert_eq!(
            Location::detect("."),
            Ok(Location::LocalDirectory {
                path: PathBuf::from(".")
            })
        );

        assert_matches!(
            Location::detect("~"),
            Ok(Location::LocalDirectory { path: _ })
        );

        assert_eq!(Location::detect("notfound"), Err(LocationError::NotFound));
    }

    #[test]
    fn test_file_url() {
        assert_eq!(
            Location::detect("file://localhost/home"),
            Ok(Location::LocalDirectory {
                path: PathBuf::from("/home")
            }),
        );

        assert_eq!(
            Location::detect("file:///home"),
            Ok(Location::LocalDirectory {
                path: PathBuf::from("/home")
            }),
        );

        assert_eq!(
            Location::detect("file://localhost/nope"),
            Err(LocationError::NotFound),
        );

        assert_eq!(
            Location::detect("file://nope/home"),
            Err(LocationError::Unsupported),
        );
    }

    #[test]
    fn test_short_git_pattern() {
        let captures = SHORT_GIT_PATTERN
            .captures("git@github.com:jimmiebfulton/archetect.git")
            .unwrap();
        assert_eq!(&captures[1], "github.com");
        assert_eq!(&captures[2], "jimmiebfulton/archetect.git");
    }
}
