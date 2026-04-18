use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio_stream::wrappers::TcpListenerStream;
use tonic::transport::{Certificate, Identity, Server, ServerTlsConfig};

use crate::errors::ArchetectError;
use crate::proto::grpc::archetect_service_server::ArchetectServiceServer as ArchetectServiceGrpcServer;
use crate::server::ArchetectServiceCore;

/// TLS configuration for `ArchetectServer`. Supply certificate + key paths to
/// terminate TLS at the server. Optional `client_ca_path` enables mutual TLS:
/// clients presenting a certificate signed by that CA are authenticated, all
/// others are rejected at the transport layer.
#[derive(Debug, Clone)]
pub struct TlsConfig {
    pub cert_path: PathBuf,
    pub key_path: PathBuf,
    pub client_ca_path: Option<PathBuf>,
}

#[derive(Clone)]
pub struct ArchetectServer {
    core: ArchetectServiceCore,
    service_port: u16,
    listener: Arc<Mutex<Option<TcpListener>>>,
    /// One-shot channel: sending (or dropping) the paired sender triggers
    /// the server's graceful shutdown. Kept inside an Option so callers
    /// who never trigger shutdown don't force the server to watch a
    /// future that never resolves.
    shutdown_signal: Arc<Mutex<Option<tokio::sync::oneshot::Receiver<()>>>>,
    shutdown_trigger: Arc<Mutex<Option<tokio::sync::oneshot::Sender<()>>>>,
    tls: Option<TlsConfig>,
}

pub struct ArchetectServerBuilder {
    core: ArchetectServiceCore,
    host: String,
    port: u16,
    tls: Option<TlsConfig>,
}

impl ArchetectServerBuilder {
    pub fn new(core: ArchetectServiceCore) -> ArchetectServerBuilder {
        ArchetectServerBuilder {
            core,
            host: "0.0.0.0".to_string(),
            port: 8080,
            tls: None,
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

    /// Enable TLS on the server. Without this the server serves plaintext
    /// (HTTP/2 cleartext / h2c), which is fine for local dev and for
    /// deployments behind a TLS-terminating reverse proxy. Supply a
    /// `TlsConfig` for end-to-end encryption.
    pub fn with_tls(mut self, tls: TlsConfig) -> Self {
        self.tls = Some(tls);
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

        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();

        Ok(ArchetectServer {
            core: self.core,
            service_port: addr.port(),
            listener: Arc::new(Mutex::new(Some(listener))),
            shutdown_signal: Arc::new(Mutex::new(Some(shutdown_rx))),
            shutdown_trigger: Arc::new(Mutex::new(Some(shutdown_tx))),
            tls: self.tls,
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

    /// Trigger graceful shutdown. The currently-running `serve()` call
    /// will stop accepting new connections, let in-flight streams finish,
    /// and return. Safe to call more than once — subsequent calls are
    /// no-ops.
    pub async fn shutdown(&self) {
        let trigger = self.shutdown_trigger.lock().await.take();
        if let Some(tx) = trigger {
            let _ = tx.send(());
        }
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

        // HTTP/2 keepalive: send a PING every 30s while a connection is
        // open and close it if no PING ACK arrives within 10s. Keeps
        // long-lived render streams alive across NATs / load balancers
        // that drop idle TCP connections.
        let mut server_builder = Server::builder()
            .http2_keepalive_interval(Some(std::time::Duration::from_secs(30)))
            .http2_keepalive_timeout(Some(std::time::Duration::from_secs(10)));

        if let Some(tls) = &self.tls {
            server_builder = server_builder
                .tls_config(build_server_tls_config(tls)?)
                .map_err(|err| {
                    ArchetectError::ServerError(format!("Failed to apply TLS config: {}", err))
                })?;
            tracing::info!(
                "TLS enabled (cert: {}, key: {}{})",
                tls.cert_path.display(),
                tls.key_path.display(),
                if tls.client_ca_path.is_some() {
                    ", mTLS"
                } else {
                    ""
                }
            );
        }

        let server = server_builder
            .add_service(health_service)
            .add_service(reflection_service)
            .add_service(ArchetectServiceGrpcServer::new(self.core.clone()));

        let shutdown_rx = self.shutdown_signal.lock().await.take();
        let shutdown_future = async move {
            match shutdown_rx {
                // Resolve when the trigger is signalled. A dropped sender
                // (the ArchetectServer was dropped without shutdown) also
                // resolves — treat that as an implicit shutdown request.
                Some(rx) => {
                    let _ = rx.await;
                }
                // No shutdown receiver (already consumed by a prior serve()
                // call) — park forever. The caller presumably knows what
                // they're doing.
                None => std::future::pending::<()>().await,
            }
        };

        server
            .serve_with_incoming_shutdown(TcpListenerStream::new(listener), shutdown_future)
            .await
            .map_err(|err| ArchetectError::ServerError(format!("Server failed: {}", err)))?;

        tracing::info!("Archetect Server shut down cleanly");
        Ok(())
    }
}

fn build_server_tls_config(tls: &TlsConfig) -> Result<ServerTlsConfig, ArchetectError> {
    let cert = fs::read(&tls.cert_path).map_err(|err| {
        ArchetectError::ServerError(format!(
            "Failed to read TLS cert '{}': {}",
            tls.cert_path.display(),
            err
        ))
    })?;
    let key = fs::read(&tls.key_path).map_err(|err| {
        ArchetectError::ServerError(format!(
            "Failed to read TLS key '{}': {}",
            tls.key_path.display(),
            err
        ))
    })?;

    let identity = Identity::from_pem(cert, key);
    let mut config = ServerTlsConfig::new().identity(identity);

    if let Some(ca_path) = &tls.client_ca_path {
        let ca = fs::read(ca_path).map_err(|err| {
            ArchetectError::ServerError(format!(
                "Failed to read client CA cert '{}': {}",
                ca_path.display(),
                err
            ))
        })?;
        config = config.client_ca_root(Certificate::from_pem(ca));
    }

    Ok(config)
}
