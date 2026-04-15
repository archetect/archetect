use std::sync::Arc;

use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio_stream::wrappers::TcpListenerStream;
use tonic::transport::Server;

use crate::errors::ArchetectError;
use crate::proto::grpc::archetect_service_server::ArchetectServiceServer as ArchetectServiceGrpcServer;
use crate::server::ArchetectServiceCore;

#[derive(Clone)]
pub struct ArchetectServer {
    core: ArchetectServiceCore,
    service_port: u16,
    listener: Arc<Mutex<Option<TcpListener>>>,
}

pub struct ArchetectServerBuilder {
    core: ArchetectServiceCore,
    host: String,
    port: u16,
}

impl ArchetectServerBuilder {
    pub fn new(core: ArchetectServiceCore) -> ArchetectServerBuilder {
        ArchetectServerBuilder {
            core,
            host: "0.0.0.0".to_string(),
            port: 8080,
        }
    }

    pub fn with_host(mut self, host: String) -> Self {
        self.host = host;
        self
    }

    pub fn with_port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    pub async fn build(self) -> Result<ArchetectServer, ArchetectError> {
        let listener = TcpListener::bind((self.host.as_str(), self.port))
            .await
            .map_err(|err| {
                ArchetectError::ServerError(format!(
                    "Failed to bind to {}:{}: {}",
                    self.host, self.port, err
                ))
            })?;
        let addr = listener.local_addr()?;

        Ok(ArchetectServer {
            core: self.core,
            service_port: addr.port(),
            listener: Arc::new(Mutex::new(Some(listener))),
        })
    }
}

impl ArchetectServer {
    pub fn builder(core: ArchetectServiceCore) -> ArchetectServerBuilder {
        ArchetectServerBuilder::new(core)
    }

    pub fn service_port(&self) -> u16 {
        self.service_port
    }

    pub async fn serve(&self) -> Result<(), ArchetectError> {
        let listener = self
            .listener
            .lock()
            .await
            .take()
            .ok_or_else(|| ArchetectError::ServerError("Server listener already consumed".to_string()))?;

        let (health_reporter, health_service) = tonic_health::server::health_reporter();
        health_reporter
            .set_serving::<ArchetectServiceGrpcServer<ArchetectServiceCore>>()
            .await;

        let reflection_service = tonic_reflection::server::Builder::configure()
            .register_encoded_file_descriptor_set(crate::proto::FILE_DESCRIPTOR_SET)
            .build_v1()
            .map_err(|err| ArchetectError::ServerError(format!("Failed to build reflection service: {}", err)))?;

        let addr = listener.local_addr()?;
        tracing::info!("Archetect Server started on {}", addr);

        let server = Server::builder()
            .add_service(health_service)
            .add_service(reflection_service)
            .add_service(ArchetectServiceGrpcServer::new(self.core.clone()));

        server
            .serve_with_incoming(TcpListenerStream::new(listener))
            .await
            .map_err(|err| ArchetectError::ServerError(format!("Server failed: {}", err)))?;

        Ok(())
    }
}
