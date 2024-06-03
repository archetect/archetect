mod server;
mod settings;
mod core;
pub mod client;
pub mod io;
mod conversion;

pub use settings::*;
pub use server::*;
pub use core::ArchetectServiceCore;

mod proto {
    tonic::include_proto!("archetect");

    pub const FILE_DESCRIPTOR_SET: &[u8] = tonic::include_file_descriptor_set!("archetect");
}
