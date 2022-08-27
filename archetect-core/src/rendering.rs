use crate::vendor::tera::{Context, Tera};
use crate::{Archetect, RenderError};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use log::{debug, error, info, trace, warn};
use crate::rules::RulesContext;
use crate::vendor::tera::extensions::create_tera;

#[derive(Clone)]
pub struct RenderContext {
    tera: Arc<Mutex<Tera>>,
    inner: Context,
    rules: RulesContext,
}

pub struct RenderContextFactory {
    tera: Arc<Mutex<Tera>>,
}

impl RenderContextFactory {
    pub fn new() -> RenderContextFactory {
        RenderContextFactory { tera: Arc::new(Mutex::new(create_tera())) }
    }

    pub fn create_context(&self) -> RenderContext {
        RenderContext::from_tera(self.tera.clone())
    }
}

impl RenderContext {
    pub fn new() -> RenderContext {
        RenderContext {
            tera: Arc::new(Mutex::new(create_tera())),
            inner: Context::new(),
            rules: RulesContext::new(),
        }
    }

    pub fn from_tera(tera: Arc<Mutex<Tera>>) -> RenderContext {
        RenderContext {
            tera,
            inner: Context::new(),
            rules: RulesContext::new(),
        }
    }

    pub fn extend(&mut self, context: &RenderContext) {
        self.inner.extend(context.inner.clone());
    }

    pub fn render_string<T: AsRef<str> + ?Sized>(&self, template: &T) -> Result<String, RenderError> {
        let template = template.as_ref();
        let mut tera = self.tera.lock().unwrap();

        match tera.render_str(template, &self.inner) {
            Ok(result) => Ok(result),
            Err(err) => {
                Err(RenderError::StringRenderError {
                    string: template.to_owned(),
                    source: err,
                })
            }
        }
    }

    pub fn insert<K: Into<String>, V: AsRef<str> + ?Sized>(&mut self, key: K, value: &V) -> Result<(), RenderError> {
        let value = self.render_string(value.as_ref())?;
        self.inner.insert(key.into(), &value);
        Ok(())
    }

    pub fn trace<T: AsRef<str> + ?Sized>(&self, template: &T) -> Result<(), RenderError> {
        trace!("{}", self.render_string(template.as_ref())?);
        Ok(())
    }

    pub fn debug<T: AsRef<str> + ?Sized>(&self, template: &T) -> Result<(), RenderError> {
        debug!("{}", self.render_string(template.as_ref())?);
        Ok(())
    }

    pub fn info<T: AsRef<str> + ?Sized>(&self, template: &T) -> Result<(), RenderError> {
        info!("{}", self.render_string(template.as_ref())?);
        Ok(())
    }

    pub fn warn<T: AsRef<str> + ?Sized>(&self, template: &T) -> Result<(), RenderError> {
        warn!("{}", self.render_string(template.as_ref())?);
        Ok(())
    }

    pub fn error<T: AsRef<str> + ?Sized>(&self, template: &T) -> Result<(), RenderError> {
        error!("{}", self.render_string(template.as_ref())?);
        Ok(())
    }

    pub fn print<T: AsRef<str> + ?Sized>(&self, template: &T) -> Result<(), RenderError> {
        println!("{}", self.render_string(template.as_ref())?);
        Ok(())
    }

    pub fn rules_context(&self) -> &RulesContext {
        &self.rules
    }
}

pub trait Renderable {
    type Result;

    fn render_legacy(&self, archetect: &mut Archetect, context: &Context) -> Result<Self::Result, RenderError>;

    fn render(&self, context: &RenderContext) -> Result<Self::Result, RenderError>;
}

impl Renderable for &str {
    type Result = String;

    fn render_legacy(&self, archetect: &mut Archetect, context: &Context) -> Result<Self::Result, RenderError> {
        archetect.render_string(&self, context)
    }

    fn render(&self, context: &RenderContext) -> Result<Self::Result, RenderError> {
        context.render_string(&self)
    }
}

impl Renderable for &String {
    type Result = String;

    fn render_legacy(&self, archetect: &mut Archetect, context: &Context) -> Result<Self::Result, RenderError> {
        archetect.render_string(&self, context)
    }

    fn render(&self, context: &RenderContext) -> Result<Self::Result, RenderError> {
        context.render_string(&self)
    }
}

impl Renderable for &Path {
    type Result = PathBuf;

    fn render_legacy(&self, archetect: &mut Archetect, context: &Context) -> Result<Self::Result, RenderError> {
        if let Some(string) = self.to_str() {
            match archetect.render_string(string, &context.clone()) {
                Ok(result) => return Ok(PathBuf::from(result)),
                Err(error) => {
                    match error {
                        RenderError::StringRenderError { string: _, source: error} => {

                            return Err(RenderError::PathRenderError {
                                path: self.into(),
                                source: error,
                            });
                        }
                        _ => panic!("Unexpected rendering error")
                    }
                }
            }
        } else {
            return Err(RenderError::InvalidPathCharacters {
                path: self.to_path_buf(),
            });
        }
    }

    fn render(&self, context: &RenderContext) -> Result<Self::Result, RenderError> {
        if let Some(string) = self.to_str() {
            match context.render_string(string) {
                Ok(result) => return Ok(PathBuf::from(result)),
                Err(error) => {
                    match error {
                        RenderError::StringRenderError { string: _, source: error} => {

                            return Err(RenderError::PathRenderError {
                                path: self.into(),
                                source: error,
                            });
                        }
                        _ => panic!("Unexpected rendering error")
                    }
                }
            }
        } else {
            return Err(RenderError::InvalidPathCharacters {
                path: self.to_path_buf(),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::rendering::Renderable;
    use crate::Archetect;
    use std::path::{Path, PathBuf};
    use crate::vendor::tera::Context;

    #[test]
    pub fn test_render_str_successfully() {
        let mut archetect = Archetect::builder().build().unwrap();
        let mut context = Context::new();
        context.insert("subject", "world");

        let render = "Hello, {{ subject }}".render_legacy(&mut archetect, &context).unwrap();
        assert_eq!(render, "Hello, world".to_owned());
    }

    #[test]
    pub fn test_render_path_successfully() {
        let mut archetect = Archetect::builder().build().unwrap();
        let mut context = Context::new();
        context.insert("parent", "hello");
        context.insert("child", "world");

        let path = Path::new("{{ parent }}/{{ child }}");

        let render = path.render_legacy(&mut archetect, &context).unwrap();
        assert_eq!(render, PathBuf::from("hello/world"));
    }

    #[test]
    pub fn test_render_empty_path() {
        let archetect = &mut Archetect::builder().build().unwrap();
        let mut context = Context::new();
        context.insert("parent", "hello");
        context.insert("child", "world");

        let path = Path::new("");
        let render = path.render_legacy(archetect, &context).unwrap();
        assert_eq!(render, PathBuf::from(String::new()));
    }
}
