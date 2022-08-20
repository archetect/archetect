use std::sync::{Arc, Mutex};
use log::{debug, error, info, trace, warn};

use rlua::{RegistryKey, Table, UserData, UserDataMethods, Variadic};
use rlua::prelude::LuaError;

use crate::RenderError;
use crate::vendor::tera::Context as TeraContext;
use crate::vendor::tera::extensions::create_tera;
use crate::vendor::tera::Tera;

#[derive(Clone, Debug)]
pub enum Type {
    String,
    Bool,
    Int,
    Enum(Variadic<String>),
    List(Box<Type>),
    Object(String)
}

impl UserData for Type { }

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

impl UserData for RenderContextFactory {
    fn add_methods<'lua, T: UserDataMethods<'lua, Self>>(methods: &mut T) {
        methods.add_method("new", |_, factory, contexts: Variadic<RenderContext>| {
            let mut result = factory.create_context();
            for context in contexts {
                result.extend(&context);
            }
            Ok(result)
        });
    }
}

#[derive(Clone)]
pub struct RenderContext {
    tera: Arc<Mutex<Tera>>,
    inner: TeraContext,
}

impl RenderContext {
    pub fn new() -> RenderContext {
        RenderContext { tera: Arc::new(Mutex::new(create_tera())), inner: TeraContext::new() }
    }

    pub fn from_tera(tera: Arc<Mutex<Tera>>) -> RenderContext {
        RenderContext { tera, inner: TeraContext::new() }
    }

    pub fn extend(&mut self, context: &RenderContext) {
        self.inner.extend(context.inner.clone());
    }

    pub fn render<T: AsRef<str> + ?Sized>(&mut self, template: &T) -> Result<String, RenderError> {
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
        let value = self.render(value)?;
        self.inner.insert(key, &value);
        Ok(())
    }
}

impl UserData for RenderContext {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method_mut("set_var", |_, context, (key, value, _options): (String, String, Variadic<Table>)| {
            let value = context.render(&value).map_err(|error| LuaError::ExternalError(Arc::new(error)))?;
            context.insert(key, &value).map_err(|error| LuaError::ExternalError(Arc::new(error)))?;
            Ok(())
        });

        methods.add_method_mut("prompt_var", |_, context, (key, options): (String, Table)| {
            let prompt = options.get::<_, String>("prompt")?;

            let default_value = if options.contains_key("default")? {
                Some(options.get::<_, String>("default")?)
            } else {
                None
            };


            let value_type = if options.contains_key("type")? {
                Some(options.get::<_, Type>("type")?)
            } else {
                None
            };

            context.insert(key, &prompt).map_err(|error| LuaError::ExternalError(Arc::new(error)))?;
            
            Ok(())
        });

        methods.add_method_mut("render", |_, context, template: String| {
            Ok(context.render(&template).map_err(|error| LuaError::ExternalError(Arc::new(error)))?)
        });

        methods.add_method_mut("info", |_, context, template: String| {
            info!("{}", context.render(&template).map_err(|error| LuaError::ExternalError(Arc::new(error)))?);
            Ok(())
        });

        methods.add_method_mut("trace", |_, context, template: String| {
            trace!("{}", context.render(&template).map_err(|error| LuaError::ExternalError(Arc::new(error)))?);
            Ok(())
        });

        methods.add_method_mut("debug", |_, context, template: String| {
            debug!("{}", context.render(&template).map_err(|error| LuaError::ExternalError(Arc::new(error)))?);
            Ok(())
        });

        methods.add_method_mut("warn", |_, context, template: String| {
            warn!("{}", context.render(&template).map_err(|error| LuaError::ExternalError(Arc::new(error)))?);
            Ok(())
        });

        methods.add_method_mut("error", |_, context, template: String| {
            error!("{}", context.render(&template).map_err(|error| LuaError::ExternalError(Arc::new(error)))?);
            Ok(())
        });

        methods.add_method_mut("print", |_, context, template: String| {
            println!("{}", context.render(&template).map_err(|error| LuaError::ExternalError(Arc::new(error)))?);
            Ok(())
        });
    }
}

#[cfg(test)]
mod tests {
    use crate::ArchetectError;
    use crate::render_context::RenderContext;

    #[test]
    fn test_render() -> Result<(), ArchetectError> {
        let mut context = RenderContext::new();
        context.insert("project-name", "Transaction Service")?;
        assert_eq!(context.render("{{ project-name | train-case }}")?, "transaction-service".to_owned());

        Ok(())
    }

    #[test]
    fn test_from_context() -> Result<(), ArchetectError> {
        let mut context = RenderContext::new();
        context.insert("firstName", "Joe")?;

        let mut child_context = RenderContext::new();
        child_context.extend(&context);
        child_context.insert("firstName", "Bobby")?;
        child_context.insert("lastName", "Sue")?;
        assert_eq!(child_context.render("{{ firstName }} {{ lastName }}").unwrap(), "Bobby Sue".to_owned());
        assert_eq!(context.render("{{ firstName }}")?, "Joe".to_owned());

        Ok(())
    }
}
