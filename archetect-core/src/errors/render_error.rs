use std::path::PathBuf;

use camino::Utf8PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error("Invalid characters in path template `{path}`")]
    InvalidPathCharacters { path: PathBuf },
    #[error("Unable to render path `{path}`: {source}")]
    PathRenderError2 {
        path: Utf8PathBuf,
        source: archetect_minijinja::Error,
    },
    #[error("Unable to render file `{path}`: {source}")]
    FileRenderIOError { path: Utf8PathBuf, source: std::io::Error },
    #[error("Rendering IO Error: {source}")]
    IOError { source: std::io::Error },
    #[error("Error copying {from} to {to}: {source}")]
    CopyError {
        from: Utf8PathBuf,
        to: Utf8PathBuf,
        source: std::io::Error,
    },
    #[error("Error writing to `{path}`: {source}")]
    WriteError { path: Utf8PathBuf, source: std::io::Error },
    #[error("Error creating file `{path}`: {source}")]
    CreateFileError { path: Utf8PathBuf, source: std::io::Error },
    #[error("Error creating directory `{path}`: {source}")]
    CreateDirectoryError { path: Utf8PathBuf, source: std::io::Error },
    #[error("Error reading `{path}`: {source}")]
    FileReadError { path: Utf8PathBuf, source: std::io::Error },
    #[error("Error reading directory in `{path}`: {source}")]
    DirectoryReadError { path: Utf8PathBuf, source: std::io::Error },
    #[error("Error listing directory `{path}`: {source}")]
    DirectoryListError { path: Utf8PathBuf, source: std::io::Error },
    #[error("Error Reading `{path}` as UTF-8")]
    Utf8ReadError { path: Utf8PathBuf },
}
