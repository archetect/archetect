use std::path::{Path, PathBuf};

use camino::{Utf8Path, Utf8PathBuf};
use rhai::EvalAltResult;

use crate::errors::ArchetectError;

pub fn to_utf8_path(path: &Path) -> &Utf8Path {
    Utf8Path::from_path(path).expect("valid UTF-8 encoded path")
}

pub fn to_utf8_path_buf(pathbuf: PathBuf) -> Utf8PathBuf {
    Utf8PathBuf::from_path_buf(pathbuf).expect("valid UTF-8 encoded path buf")
}

pub(crate) fn restrict_path_manipulation(path: &str) -> Result<&str, Box<EvalAltResult>> {
    if path.starts_with("~/") || path.starts_with("../") || path.contains("/../") || path.ends_with("/..") {
        return Err(Box::new(EvalAltResult::ErrorSystem(
            "Rendering Error".into(),
            Box::new(ArchetectError::GeneralError(
                "Paths in Rhai scripts may not contain path manipulation patterns".into(),
            )),
        )));
    }
    Ok(path)
}
