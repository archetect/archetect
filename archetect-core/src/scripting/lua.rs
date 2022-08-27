use std::fs;
use std::sync::{Arc, Mutex};
use rlua::{Lua, Table, UserData, UserDataMethods, Variadic};
use rlua::prelude::LuaError;
use uuid::Uuid;
use crate::{Archetect, ArchetectError, Archetype};
use crate::rendering::{RenderContext, RenderContextFactory};

pub struct LuaScriptContext {
    archetect: Arc<Mutex<Archetect>>,
    archetype: Arc<Mutex<Archetype>>,
}

impl LuaScriptContext {
    pub fn new(archetect: Arc<Mutex<Archetect>>, archetype: Arc<Mutex<Archetype>>) -> LuaScriptContext {
        LuaScriptContext { archetect, archetype }
    }

    pub fn execute_string(&self, script: &str) -> Result<(), LuaError> {
        let lua = Lua::new();

        self.bootstrap(&lua)?;

        lua.context(|ctx| {

            ctx.load(script)
                .exec()?;
            Ok(())
        })?;

        Ok(())
    }

    pub fn execute_archetype(&self, archetype: &Archetype) -> Result<(), ArchetectError> {
        let script_path = archetype.source().directory().join(archetype.configuration().script().unwrap());

        let script = fs::read_to_string(&script_path)?;

        let lua = Lua::new();
        self.bootstrap(&lua)?;
        lua.context(|ctx| {
            ctx.load(&script)
                .set_name("archetype.lua")?
                .exec()
        })?;

        Ok(())
    }

    fn bootstrap<'lua>(&self, lua: &Lua) -> Result<(), LuaError> {
        lua.context(|ctx| {
            let globals = ctx.globals();

            // Register Context API
            globals.set("Context", RenderContextFactory::new())?;

            // Register Type API
            globals.set("String", Type::String)?;
            globals.set("Int", Type::Int)?;
            globals.set("Bool", Type::Bool)?;
            let enum_constructor = ctx.create_function(|_, values: Variadic<String>| Ok(Type::Enum(values)))?;
            globals.set("Enum", enum_constructor)?;
            let list_constructor = ctx.create_function(|_, value: Type| {
                Ok(Type::List(Box::new(value)))
            })?;
            globals.set("List", list_constructor)?;
            let object_constructor = ctx.create_function(|ctx, table: Table| {
                let key = Uuid::new_v4().to_string();
                ctx.set_named_registry_value(&key, table)?;
                Ok(Type::Object(key))
            })?;
            globals.set("Object", object_constructor)?;

            Ok(())
        })?;

        Ok(())
    }
}


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



impl UserData for RenderContext {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method_mut("set_var", |_, context, (key, value, _options): (String, String, Variadic<Table>)| {
            let value = context.render_string(&value).map_err(|error| LuaError::ExternalError(Arc::new(error)))?;
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


            let _value_type = if options.contains_key("type")? {
                Some(options.get::<_, Type>("type")?)
            } else {
                None
            };

            context.insert(key, &prompt).map_err(|error| LuaError::ExternalError(Arc::new(error)))?;

            Ok(())
        });

        methods.add_method_mut("render", |_, context, template: String| {
            Ok(context.render_string(&template).map_err(|error| LuaError::ExternalError(Arc::new(error)))?)
        });

        methods.add_method_mut("trace", |_, context, template: String| {
            context.trace(&template).map_err(|error| LuaError::ExternalError(Arc::new(error)))?;
            Ok(())
        });

        methods.add_method_mut("debug", |_, context, template: String| {
            context.debug(&template).map_err(|error| LuaError::ExternalError(Arc::new(error)))?;
            Ok(())
        });

        methods.add_method_mut("info", |_, context, template: String| {
            context.info(&template).map_err(|error| LuaError::ExternalError(Arc::new(error)))?;
            Ok(())
        });

        methods.add_method_mut("warn", |_, context, template: String| {
            context.warn(&template).map_err(|error| LuaError::ExternalError(Arc::new(error)))?;
            Ok(())
        });


        methods.add_method_mut("error", |_, context, template: String| {
            context.error(&template).map_err(|error| LuaError::ExternalError(Arc::new(error)))?;
            Ok(())
        });

        methods.add_method_mut("print", |_, context, template: String| {
            context.print(&template).map_err(|error| LuaError::ExternalError(Arc::new(error)))?;
            Ok(())
        });
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use crate::{ArchetectError, Archetype};
    use std::sync::{Arc, Mutex};
    use crate::Archetect;
    use crate::rendering::RenderContext;
    use crate::scripting::lua::LuaScriptContext;
    use crate::source::Source;

    #[test]
    fn test_render() -> Result<(), ArchetectError> {
        let mut context = RenderContext::new();
        context.insert("project-name", "Transaction Service")?;
        assert_eq!(context.render_string("{{ project-name | train-case }}")?, "transaction-service".to_owned());

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
        assert_eq!(child_context.render_string("{{ firstName }} {{ lastName }}").unwrap(), "Bobby Sue".to_owned());
        assert_eq!(context.render_string("{{ firstName }}")?, "Joe".to_owned());

        Ok(())
    }

    #[test]
    fn test_set_var() ->  anyhow::Result<()>{
        let archetect = Archetect::build().unwrap();
        let archetype = Archetype::from_source(&Source::LocalDirectory { path: PathBuf::new() })?;
        let script_context = LuaScriptContext::new(
            Arc::new(Mutex::new(archetect)),
            Arc::new(Mutex::new(archetype)),
        );

        script_context.execute_string(r#"
local context = Context:new()
context:set_var("project-name", "Transaction Service")
context:info("We've got a new {{ project-name }}!")

print("Hello, World!")

print(context:render("{{ 'tech.nirvana' | package_to_directory }}"))

        "#).unwrap();

        Ok(())
    }

    #[test]
    fn test_prompt_var() -> anyhow::Result<()> {
        let archetect = Archetect::build().unwrap();
        let archetype = Archetype::from_source(&Source::LocalDirectory { path: PathBuf::new() })?;
        let script_context = LuaScriptContext::new(
            Arc::new(Mutex::new(archetect)),
            Arc::new(Mutex::new(archetype)),
        );

        script_context.execute_string(r#"
local context = Context:new()

context:prompt_var("project-name", {
    prompt = "Project Name:",
})

context:prompt_var("project-suffix", {
    prompt = "Project Suffix:",
    default = "Service",
    type = String,
})

context:prompt_var("feature-sqs", {
    prompt = "[Feature] SQS:",
    default = "y",
    type = Bool,
})

context:prompt_var("persistence", {
    prompt = "Persistence:",
    type = Enum("CockroachDB", "PostgreSQL", "DynamoDB"),
})

context:prompt_var("persistence2", {
    prompt = "Persistence:",
    type = List(String),
})

context:prompt_var("model", {
    prompt = "Model:",
    type = Object({
        fields = {
            type = String,
        },
    }),
})

print("Hello, World!")

print(context:render("{{ project-name }}"))

        "#).unwrap();

        Ok(())
    }

}


