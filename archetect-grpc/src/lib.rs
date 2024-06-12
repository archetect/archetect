pub use core::ArchetectServiceCore;
pub use server::*;

pub mod client;
mod conversion;
mod core;
pub mod io;
mod server;

mod proto {
    tonic::include_proto!("archetect");

    pub const FILE_DESCRIPTOR_SET: &[u8] = tonic::include_file_descriptor_set!("archetect");
}
