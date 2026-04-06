pub mod grpc {
    tonic::include_proto!("archetect");

    pub const FILE_DESCRIPTOR_SET: &[u8] = tonic::include_file_descriptor_set!("archetect");
}

mod conversions;

pub use grpc::archetect_service_client;
pub use grpc::archetect_service_server;
pub use grpc::FILE_DESCRIPTOR_SET;
