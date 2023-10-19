use camino::Utf8PathBuf;
use std::fmt::{Display, Formatter};
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    InvalidPathCharacters {
        path: PathBuf,
    },
    PathRenderError2 {
        path: PathBuf,
        source: minijinja::Error,
    },
    FileRenderIOError {
        path: Utf8PathBuf,
        source: std::io::Error,
    },
    IOError {
        #[from]
        source: std::io::Error,
    },
}

impl Display for RenderError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            RenderError::InvalidPathCharacters { path } => {
                write!(f, "Invalid characters in path template `{:?}`", path)
            }
            RenderError::PathRenderError2 { path, source } => {
                write!(f, "Unable to render path `{:?}`: {}", path, source)
            }
            RenderError::FileRenderIOError { path, source } => {
                write!(f, "Unable to render file `{:?}`: {}", path, source)
            }
            RenderError::IOError { source } => {
                write!(f, "Rendering IO Error: {}", source)
            }
        }
    }
}
