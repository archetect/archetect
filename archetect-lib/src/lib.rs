#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate pest_derive;
#[cfg(test)]
#[macro_use]
extern crate pretty_assertions;
#[macro_use]
extern crate serde_derive;
#[cfg_attr(test, macro_use)]
extern crate serde_json;

mod core;
mod errors;
pub mod heck;

pub mod archetype;
pub mod config;
pub mod input;
pub mod system;
pub mod template_engine;
pub mod util;
pub mod serde_utils;

pub use crate::archetype::{Archetype, ArchetypeError};
pub use crate::core::{Archetect};
pub use crate::errors::{ArchetectError, RenderError};

