use camino::{Utf8Path, Utf8PathBuf};
use std::path::{Path, PathBuf};

pub fn to_utf8_path(path: &Path) -> &Utf8Path {
    Utf8Path::from_path(path).expect("valid UTF-8 encoded path")
}

pub fn to_utf8_path_buf(pathbuf: PathBuf) -> Utf8PathBuf {
    Utf8PathBuf::from_path_buf(pathbuf).expect("valid UTF-8 encoded path buf")
}
