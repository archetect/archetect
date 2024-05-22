use std::sync::Arc;

use anyhow::Result;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio_stream::wrappers::TcpListenerStream;
use tonic::transport::Server;

use crate::core::ArchetectServiceCore;
use crate::proto::archetect_service_server::ArchetectServiceServer as ArchetectServiceGrpcServer;
use crate::settings::ServerSettings;

#[derive(Clone)]
pub struct ArchetectServer {
    core: ArchetectServiceCore,
    service_port: u16,
    listener: Arc<Mutex<Option<TcpListener>>>,
}

pub struct Builder {
    settings: ServerSettings,
    core: ArchetectServiceCore,
}

impl Builder {
    pub fn new(core: ArchetectServiceCore) -> Builder {
        Builder {
            settings: ServerSettings::default(),
            core,
        }
    }

    pub fn with_settings(mut self, settings: &ServerSettings) -> Builder {
        self.settings = settings.clone();
        self
    }

    pub fn with_random_port(mut self) -> Builder {
        self.settings.service_mut().set_port(0);
        self
    }

    pub async fn build(self) -> Result<ArchetectServer> {
        let listener = TcpListener::bind((self.settings.host(), self.settings.service().port())).await?;
        let addr = listener.local_addr()?;

        Ok(ArchetectServer {
            core: self.core,
            service_port: addr.port(),
            listener: Arc::new(Mutex::new(Some(listener))),
        })
    }
}

impl ArchetectServer {
    pub fn builder(core: ArchetectServiceCore) -> Builder {
        Builder::new(core)
    }

    pub fn service_port(&self) -> u16 {
        self.service_port
    }

    pub async fn serve(&self) -> Result<()> {
        let listener = self.listener.lock().await.take().expect("Listener Expected");

        let (mut health_reporter, health_service) = tonic_health::server::health_reporter();
        health_reporter
            .set_serving::<ArchetectServiceGrpcServer<ArchetectServiceCore>>()
            .await;

        let reflection_service = tonic_reflection::server::Builder::configure()
            .register_encoded_file_descriptor_set(crate::proto::FILE_DESCRIPTOR_SET)
            .build()
            .unwrap();

        let server = Server::builder()
            .add_service(health_service)
            .add_service(reflection_service)
            .add_service(ArchetectServiceGrpcServer::new(self.core.clone()));

        tracing::info!("StreamingService started on {}", listener.local_addr()?);

        server.serve_with_incoming(TcpListenerStream::new(listener)).await?;

        Ok(())
    }
}
