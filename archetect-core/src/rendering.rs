use crate::template_engine::Context;
use crate::{Archetect, RenderError};
use std::path::{Path, PathBuf};

pub trait Renderable {
    type Result;

    fn render(&self, archetect: &Archetect, context: &Context) -> Result<Self::Result, RenderError>;
}

impl Renderable for &str {
    type Result = String;

    fn render(&self, archetect: &Archetect, context: &Context) -> Result<Self::Result, RenderError> {
        archetect.render_string(&self, context)
    }
}

impl Renderable for &String {
    type Result = String;

    fn render(&self, archetect: &Archetect, context: &Context) -> Result<Self::Result, RenderError> {
        archetect.render_string(&self, context)
    }
}

impl Renderable for &Path {
    type Result = PathBuf;

    fn render(&self, archetect: &Archetect, context: &Context) -> Result<Self::Result, RenderError> {
        if let Some(string) = self.to_str() {
            match archetect.template_engine().render_string(string, context.clone()) {
                Ok(result) => return Ok(PathBuf::from(result)),
                Err(error) => {
                    return Err(RenderError::PathRenderError {
                        source: self.into(),
                        error,
                        message: String::new(),
                    });
                }
            }
        } else {
            return Err(RenderError::InvalidPathCharacters {
                source: self.to_path_buf(),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::rendering::Renderable;
    use crate::template_engine::Context;
    use crate::Archetect;
    use std::path::{Path, PathBuf};

    #[test]
    pub fn test_render_str_successfully() {
        let archetect = Archetect::builder().build().unwrap();
        let mut context = Context::new();
        context.insert("subject", "world");

        let render = "Hello, {{ subject }}".render(&archetect, &context).unwrap();
        assert_eq!(render, "Hello, world".to_owned());
    }

    #[test]
    pub fn test_render_path_successfully() {
        let archetect = Archetect::builder().build().unwrap();
        let mut context = Context::new();
        context.insert("parent", "hello");
        context.insert("child", "world");

        let path = Path::new("{{ parent }}/{{ child }}");

        let render = path.render(&archetect, &context).unwrap();
        assert_eq!(render, PathBuf::from("hello/world"));
    }

    #[test]
    pub fn test_render_empty_path() {
        let archetect = Archetect::builder().build().unwrap();
        let mut context = Context::new();
        context.insert("parent", "hello");
        context.insert("child", "world");

        let path = Path::new("");
        let render = path.render(&archetect, &context).unwrap();
        assert_eq!(render, PathBuf::from(String::new()));
    }
}
