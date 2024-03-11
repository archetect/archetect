use std::fmt::{Display, Formatter};
use rhai::{CustomType, Dynamic, Engine, FnNamespace, FuncRegistration, Module, TypeBuilder};


pub(crate) fn register(
    engine: &mut Engine,
) {
     let mut module = Module::new();
     FuncRegistration::new("Pair")
        .with_namespace(FnNamespace::Internal)
        .with_purity(true)
        .with_volatility(false)
        .set_into_module(&mut module, |key: &str, value: Dynamic| Pair(key.to_string(), value.clone()));
    engine.build_type::<Pair>();
    engine.register_global_module(module.into());
}

#[derive(Clone, Debug)]
pub struct Pair(String, Dynamic);

impl Pair {
    pub fn key(&mut self) -> &String {
        &self.0
    }

    pub fn value(&mut self) -> &Dynamic {
        &self.1
    }
}

impl CustomType for Pair {
    fn build(mut builder: TypeBuilder<Self>) {
        builder.with_name("Pair")
            .with_get("key", |pair: &mut Pair|pair.key().to_string())
            .with_get("value", |pair: &mut Pair| pair.value().clone())
            .with_fn("to_string", |pair: &mut Pair| pair.to_string())
            .with_fn("to_debug", |pair: &mut Pair| format!("{pair:?}"))
        ;
    }
}

impl Display for Pair {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Pair(\"{}\", {})", self.0, self.1)
    }
}