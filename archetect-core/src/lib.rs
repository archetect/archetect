#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate pest_derive;
#[cfg(test)]
#[macro_use]
extern crate pretty_assertions;
#[macro_use]
extern crate serde_derive;

pub use crate::core::Archetect;

mod core;

pub mod archetype;
pub mod config;
pub mod errors;
pub mod requirements;
pub mod system;
mod utils;
pub mod v2;
