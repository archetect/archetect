use std::cell::RefCell;
use std::error::Error;
use std::fs;
use std::rc::Rc;
use rhai::{Dynamic, Engine, EvalAltResult, Scope};
use crate::{ArchetectError, Archetype, RenderError};
use crate::rendering::RenderContext;

pub struct RhaiScriptContext {

}

impl RhaiScriptContext {
    pub fn new() -> RhaiScriptContext {
        RhaiScriptContext {
            
        }
    }

    pub fn execute(&self, archetype: &Archetype) -> Result<(), ArchetectError> {
        let script_path = archetype.source().directory().join(archetype.configuration().script().unwrap());

        let mut engine = Engine::new();
        let mut scope = Scope::new();

        self.bootstrap(&mut engine, &mut scope);


        engine.run_file_with_scope(&mut scope, script_path)?;

        Ok(())
    }

    fn bootstrap(&self, engine: &mut Engine, scope: &mut Scope) {

        engine.register_type_with_name::<Context>("Context")
            .register_fn("Context", Context::new)
            .register_fn("Context", Context::new_from_context)
            .register_fn("set_var", Context::set_var)
            .register_fn("traceln", Context::trace)
            .register_fn("debugln", Context::debug)
            .register_fn("infoln", Context::info)
            .register_fn("warnln", Context::warn)
            .register_fn("errorln", Context::error)
            .register_fn("println", Context::println)
        ;

        engine.register_type_with_name::<Type>("Type");
        engine.register_fn("List", |typ: Type| {
            Type::List(Box::new(typ))
        });

        scope.push_constant("String", Type::String);
        scope.push_constant("Int", Type::Int);
        scope.push_constant("Bool", Type::Bool);
    }
}

#[derive(Clone)]
pub enum Type {
    String,
    Int,
    Bool,
    Enum(Vec<String>),
    List(Box<Type>),
    Object(Dynamic),
}

#[derive(Clone)]
pub struct Context {
    inner: Rc<RefCell<RenderContext>>,
}

impl Context {
    pub fn new() -> Context {
        Context { inner: Rc::new(RefCell::new(RenderContext::new())) }
    }

    pub fn new_from_context(context: &mut Context) -> Context {
        Context { inner: Rc::new(RefCell::new(context.inner.borrow().clone())) }
    }

    pub fn set_var(&mut self, key: &str, value: &str) {
        self.inner.borrow_mut().insert(key, value).unwrap();
    }

    pub fn trace(&mut self, template: &str) {
        self.inner.borrow().trace(template).unwrap();
    }

    pub fn debug(&mut self, template: &str) {
        self.inner.borrow().debug(template).unwrap();
    }

    pub fn info(&mut self, template: &str) {
        self.inner.borrow().info(template).unwrap();
    }

    pub fn warn(&mut self, template: &str) {
        self.inner.borrow().warn(template).unwrap();
    }

    pub fn error(&mut self, template: &str) {
        self.inner.borrow().error(template).unwrap();
    }

    pub fn println(&mut self, template: &str) {
        self.inner.borrow().print(template).unwrap();
    }
}
