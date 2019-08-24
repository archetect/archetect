use std::path::PathBuf;

#[derive(Debug)]
pub enum RenderError {
    PathRenderError{ source: PathBuf, error: crate::template_engine::Error, message: String },
    FileRenderError{ source: PathBuf, error: crate::template_engine::Error, message: String },
    IOError { error: std::io::Error, message: String },
}

impl From<std::io::Error> for RenderError {
    fn from(error: std::io::Error) -> Self {
        let message = error.to_string();
        RenderError::IOError { error, message }
    }
}