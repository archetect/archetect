use std::path::{Path, PathBuf};

use camino::{Utf8Path, Utf8PathBuf};
use rhai::{EvalAltResult, NativeCallContext};

use crate::errors::{ArchetypeScriptError, ArchetypeScriptErrorWrapper};

pub fn to_utf8_path(path: &Path) -> &Utf8Path {
    Utf8Path::from_path(path).expect("valid UTF-8 encoded path")
}

pub fn to_utf8_path_buf(pathbuf: PathBuf) -> Utf8PathBuf {
    Utf8PathBuf::from_path_buf(pathbuf).expect("valid UTF-8 encoded path buf")
}

pub(crate) fn restrict_path_manipulation<'a, 'b>(call: &'a NativeCallContext, path: &'b str) -> Result<&'b str, Box<EvalAltResult>> {
    if path.starts_with("~/") || path.starts_with("../") || path.contains("/../") || path.ends_with("/..") {
        return Err(ArchetypeScriptErrorWrapper(call, ArchetypeScriptError::PathManipulationError {
            path: path.to_string()
        }).into());
    }
    Ok(path)
}
