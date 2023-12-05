#[cfg(test)]
#[macro_use]
extern crate pretty_assertions;
#[macro_use]
extern crate serde_derive;

pub use crate::core::Archetect;

mod core;

pub mod archetype;
pub mod catalog;
pub mod configuration;
pub mod errors;
pub mod runtime;
pub mod script;
pub mod source;
pub mod system;
mod utils;
