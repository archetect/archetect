use crate::errors::{ArchetectError, ArchetypeError};
use camino::{Utf8Path, Utf8PathBuf};
use rhai::{EvalAltResult, NativeCallContext};
use std::path::{Path, PathBuf};

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

pub struct ArchetectRhaiFunctionError<'a>(pub &'a str, pub &'a NativeCallContext<'a>, pub ArchetectError);

impl<'a> From<ArchetectRhaiFunctionError<'a>> for Box<EvalAltResult> {
    fn from(value: ArchetectRhaiFunctionError<'a>) -> Self {
        let fn_name = value.1.fn_name().to_owned();
        let source = value
            .1
            .source()
            .unwrap_or_else(|| value.1.global_runtime_state().source().unwrap_or("<unknown>"));
        let position = value.1.position();
        let error = EvalAltResult::ErrorSystem(value.0.to_string(), Box::new(value.2));

        Box::new(EvalAltResult::ErrorInFunctionCall(
            fn_name,
            source.to_string(),
            Box::new(error),
            position,
        ))
    }
}

pub struct ArchetypeRhaiFunctionError<'a>(pub &'a str, pub &'a NativeCallContext<'a>, pub ArchetypeError);

impl<'a> From<ArchetypeRhaiFunctionError<'a>> for Box<EvalAltResult> {
    fn from(value: ArchetypeRhaiFunctionError<'a>) -> Self {
        let fn_name = value.1.fn_name().to_owned();
        let source = value
            .1
            .source()
            .unwrap_or_else(|| value.1.global_runtime_state().source().unwrap_or("<unknown>"));
        let position = value.1.position();
        let error = EvalAltResult::ErrorSystem(value.0.to_string(), Box::new(value.2));

        Box::new(EvalAltResult::ErrorInFunctionCall(
            fn_name,
            source.to_string(),
            Box::new(error),
            position,
        ))
    }
}

pub struct ArchetectRhaiSystemError<'a>(pub &'a str, pub ArchetectError);

impl<'a> From<ArchetectRhaiSystemError<'a>> for Box<EvalAltResult> {
    fn from(value: ArchetectRhaiSystemError<'a>) -> Self {
        Box::new(EvalAltResult::ErrorSystem(value.0.to_string(), Box::new(value.1)))
    }
}

pub struct ArchetypeRhaiSystemError<'a>(pub &'a str, pub ArchetypeError);

impl<'a> From<ArchetypeRhaiSystemError<'a>> for Box<EvalAltResult> {
    fn from(value: ArchetypeRhaiSystemError<'a>) -> Self {
        Box::new(EvalAltResult::ErrorSystem(value.0.to_string(), Box::new(value.1)))
    }
}
