use std::path::PathBuf;

pub enum RenderError {
    PathRenderError{ source: PathBuf, message: String },
    FileRenderError{ source: PathBuf, message: String },
    IOError { message: String },
}