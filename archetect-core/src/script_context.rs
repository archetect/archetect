use std::fs;
use std::sync::{Arc, Mutex};
use rlua::{Lua, Table, Variadic};
use rlua::prelude::LuaError;
use thiserror::private::DisplayAsDisplay;
use uuid::Uuid;
use crate::{Archetect, ArchetectError, Archetype};
use crate::render_context::{RenderContext, RenderContextFactory, Type};

pub struct ScriptContext {
    archetect: Arc<Mutex<Archetect>>,
}

impl ScriptContext {
    pub fn new(archetect: Arc<Mutex<Archetect>>) -> ScriptContext {
        ScriptContext { archetect }
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
        let script_path = archetype.source().directory().join("archetype.lua");

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

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};
    use crate::Archetect;
    use crate::script_context::ScriptContext;

    #[test]
    fn test_set_var() {
        let archetect = Archetect::build().unwrap();
        let script_context = ScriptContext::new(Arc::new(Mutex::new(archetect)));

        script_context.execute_string(r#"
local context = Context:new()
context:set_var("project-name", "Transaction Service")
context:info("We've got a new {{ project-name }}!")

print("Hello, World!")

print(context:render("{{ 'tech.nirvana' | package_to_directory }}"))

        "#).unwrap();
    }

    #[test]
    fn test_prompt_var() {
        let archetect = Archetect::build().unwrap();
        let script_context = ScriptContext::new(Arc::new(Mutex::new(archetect)));

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
    }

}


