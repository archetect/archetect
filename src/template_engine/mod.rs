
#[macro_use]
mod macros;
mod builtins;
mod context;
mod errors;
mod parser;
mod renderer;
mod sort_utils;
mod template;
mod tera;
mod utils;

// Library exports.

// Template is meant to be used internally only but is exported for test/bench.
pub use crate::template_engine::builtins::filters::Filter;
pub use crate::template_engine::builtins::functions::Function;
pub use crate::template_engine::builtins::testers::Test;
pub use crate::template_engine::context::Context;
pub use crate::template_engine::errors::{Error, ErrorKind, Result};
#[doc(hidden)]
pub use crate::template_engine::template::Template;
pub use crate::template_engine::tera::Tera;
pub use crate::template_engine::utils::escape_html;
/// Re-export Value and other useful things from serde
/// so apps/tools can encode data in Tera types
pub use serde_json::value::{from_value, to_value, Map, Number, Value};

// Exposes the AST if one needs it but changing the AST is not considered
// a breaking change so it isn't public
#[doc(hidden)]
pub use crate::template_engine::parser::ast;
