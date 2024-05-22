mod server;
mod settings;
mod core;

pub use settings::*;
pub use server::*;
pub use core::*;

mod proto {
    tonic::include_proto!("archetect");

    pub const FILE_DESCRIPTOR_SET: &[u8] = tonic::include_file_descriptor_set!("archetect");
}
