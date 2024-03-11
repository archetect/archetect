use std::fmt::{Display, Formatter};
use camino::{Utf8Path, Utf8PathBuf};
use log::{error, warn};
use rhai::{CustomType, Engine, EvalAltResult, FnNamespace, FuncRegistration, Module, NativeCallContext, TypeBuilder};

use crate::archetype::render_context::RenderContext;
use crate::utils::restrict_path_manipulation;

pub(crate) fn register(
    engine: &mut Engine,
    render_context: RenderContext,
) {
    let mut module = Module::new();
    let func = move |call: NativeCallContext, path: &str| {
        create_path(&call, render_context.clone(), path.to_string())
    };
    FuncRegistration::new("Path")
        .with_namespace(FnNamespace::Internal)
        .with_purity(true)
        .with_volatility(false)
        .set_into_module(&mut module, func);
    engine.build_type::<Path>();
    engine.register_global_module(module.into());
}

pub fn create_path(call: &NativeCallContext, render_context: RenderContext, path: String) -> Result<Path, Box<EvalAltResult>> {
    let path = restrict_path_manipulation(call, &path)?;
    Ok(Path::new(path.to_string(), render_context))
}

#[derive(Clone, Debug)]
pub struct Path {
    path: String,
    full_path: Utf8PathBuf
}

impl Path {
    pub fn new(path: String, render_context: RenderContext) -> Path {
        Path {
            path: path.clone(),
            full_path: render_context.destination().join(&path),
        }
    }

    pub fn path(&mut self) -> &str {
        &self.path
    }

    pub fn full_path(&mut self) -> &Utf8Path {
        &self.full_path
    }

    pub fn exists(&mut self) -> bool {
        self.full_path.exists()
    }

    //noinspection RsSelfConvention
    pub fn is_file(&mut self) -> bool {
        self.full_path.is_file()
    }

    //noinspection RsSelfConvention
    pub fn is_dir(&mut self) -> bool {
        self.full_path.is_dir()
    }

    pub fn remove(&mut self) {
        if self.full_path.is_file() {
            match std::fs::remove_file(&self.full_path) {
                Ok(_) => {}
                Err(err) => {
                    error!("Error deleting file {}: {}", self.full_path.to_string(), err);
                }
            }
        } else if self.full_path.is_dir() {
            match std::fs::remove_dir_all(&self.full_path) {
                Ok(_) => {}
                Err(err) => {
                    error!("Error deleting directory {}: {}", self.full_path.to_string(), err);
                }
            }
        } else {
            warn!("Attempting to delete path '{}', but it does not exist", self.path);
        }
    }

}

impl CustomType for Path {
    fn build(mut builder: TypeBuilder<Self>) {
         builder
             .with_name("Path")
             .with_fn("exists", Path::exists)
             .with_fn("is_file", Path::is_file)
             .with_fn("is_dir", Path::is_dir)
             .with_fn("delete", Path::remove)
             .with_fn("remove", Path::remove)
             .with_fn("path", |destination: &mut Path| destination.path().to_string())
             .with_fn("full_path", |destination: &mut Path| destination.full_path().to_string())
             .with_fn("to_debug", |destination: &mut Path| destination.to_string())
             .with_fn("to_string", |destination: &mut Path| destination.to_string())
        ;
    }
}

impl Display for Path {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Path(\"{}\")", self.path)
    }
}