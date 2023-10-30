#[macro_use]
extern crate lazy_static;

#[cfg(test)]
#[macro_use]
extern crate pretty_assertions;
#[macro_use]
extern crate serde_derive;

pub use crate::core::Archetect;

mod core;

pub mod configuration;
pub mod errors;
pub mod source;
pub mod system;
mod utils;
pub mod v2;
