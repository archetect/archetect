use std::path::{Path, PathBuf};
use camino::{Utf8Path, Utf8PathBuf};

pub fn to_utf8_path(path: &Path) -> &Utf8Path {
    Utf8Path::from_path(path)
        .expect("valid UTF-8 encoded path")
}

pub fn to_utf8_path_buf(pathbuf: PathBuf) -> Utf8PathBuf {
    Utf8PathBuf::from_path_buf(pathbuf)
        .expect("valid UTF-8 encoded path buf")
}

#[cfg(test)]
pub mod testing {
    pub fn strip_newline(input: &str) -> &str {
        input
            .strip_suffix("\r\n")
            .or(input.strip_suffix("\n"))
            .unwrap_or(input)
    }
}
