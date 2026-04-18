use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use tonic::transport::{Certificate, Channel, ClientTlsConfig, Endpoint, Identity};
use tracing::{debug, warn};

use archetect_api::ClientMessage;
use archetect_terminal_io::TerminalClient;

use crate::archetype::render_context::RenderContext;
use crate::errors::ArchetectError;
use crate::io::AsyncClientIoHandle;
use crate::proto::grpc;
use crate::proto::grpc::archetect_service_client::ArchetectServiceClient;
use crate::proto::grpc::script_message::Message;

/// Client-side TLS configuration. `ca_cert_path` overrides the default trust
/// store (useful for self-signed server certs). `client_cert_path` +
/// `client_key_path` are supplied together to authenticate the client under
/// mutual TLS. `domain_name` overrides the SNI + certificate verification
/// name, which is useful when the server's cert CN doesn't match the
/// endpoint host (common in dev setups).
#[derive(Debug, Clone, Default)]
pub struct ClientTlsOptions {
    pub ca_cert_path: Option<PathBuf>,
    pub client_cert_path: Option<PathBuf>,
    pub client_key_path: Option<PathBuf>,
    pub domain_name: Option<String>,
}

/// Tunable knobs for the client. Defaults are conservative enough for local
/// use and short-lived RPCs; production deployments should increase them via
/// CLI flags.
#[derive(Debug, Clone)]
pub struct ClientOptions {
    /// Per-TCP-connect timeout applied on every connection attempt.
    pub connect_timeout: Duration,
    /// Cap on total wall-clock time spent (re)connecting before giving up.
    pub max_connect_retries: u32,
    /// Base backoff applied between connect attempts. Actual backoff grows
    /// exponentially (base, base*2, base*4, ...) capped at
    /// `max_backoff`.
    pub connect_backoff_base: Duration,
    /// Upper bound for the exponential backoff between retries.
    pub max_backoff: Duration,
    /// HTTP/2 keepalive PING interval. `None` disables keepalive.
    pub http2_keepalive_interval: Option<Duration>,
    /// HTTP/2 keepalive PING ACK timeout before the connection is closed.
    pub http2_keepalive_timeout: Option<Duration>,
    /// TLS configuration. `None` means plaintext. When `Some`, any fields
    /// inside are additionally optional — an empty `ClientTlsOptions`
    /// uses the system trust store with the endpoint's hostname as SNI.
    pub tls: Option<ClientTlsOptions>,
}

impl Default for ClientOptions {
    fn default() -> Self {
        Self {
            connect_timeout: Duration::from_secs(5),
            max_connect_retries: 5,
            connect_backoff_base: Duration::from_millis(250),
            max_backoff: Duration::from_secs(5),
            // Mirror the server defaults: 30s PING interval, 10s timeout.
            http2_keepalive_interval: Some(Duration::from_secs(30)),
            http2_keepalive_timeout: Some(Duration::from_secs(10)),
            tls: None,
        }
    }
}

pub fn start(render_context: RenderContext, endpoint: String) -> Result<(), ArchetectError> {
    start_with_options(render_context, endpoint, ClientOptions::default())
}

pub fn start_with_options(
    render_context: RenderContext,
    endpoint: String,
    options: ClientOptions,
) -> Result<(), ArchetectError> {
    start_remote(render_context, endpoint, String::new(), options)
}

/// Render a specific catalog path on a remote server. `catalog_path` is
/// slash-separated (e.g. "services/grpc"); empty string asks the server
/// to render its default entry.
pub fn start_remote(
    render_context: RenderContext,
    endpoint: String,
    catalog_path: String,
    options: ClientOptions,
) -> Result<(), ArchetectError> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|err| {
            ArchetectError::ServerError(format!("Failed to start Tokio runtime: {}", err))
        })?;

    runtime
        .block_on(start_async(render_context, endpoint, catalog_path, options))
        .map_err(|err: anyhow::Error| ArchetectError::ServerError(format!("Client connection error: {}", err)))?;

    Ok(())
}

async fn connect_with_retry(
    endpoint: String,
    options: &ClientOptions,
) -> anyhow::Result<ArchetectServiceClient<Channel>> {
    let mut endpoint_builder = Endpoint::from_shared(endpoint.clone())?
        .connect_timeout(options.connect_timeout);
    if let Some(interval) = options.http2_keepalive_interval {
        endpoint_builder = endpoint_builder.http2_keep_alive_interval(interval);
    }
    if let Some(timeout) = options.http2_keepalive_timeout {
        endpoint_builder = endpoint_builder.keep_alive_timeout(timeout);
    }
    if let Some(tls) = &options.tls {
        endpoint_builder = endpoint_builder.tls_config(build_client_tls_config(tls)?)?;
    }

    let mut attempt: u32 = 0;
    loop {
        attempt += 1;
        match endpoint_builder.connect().await {
            Ok(channel) => return Ok(ArchetectServiceClient::new(channel)),
            Err(err) => {
                if attempt > options.max_connect_retries {
                    return Err(anyhow::anyhow!(
                        "failed to connect to {} after {} attempts: {}",
                        endpoint,
                        attempt,
                        err
                    ));
                }
                // Exponential backoff capped at max_backoff. 2^(n-1) * base.
                let shift = (attempt - 1).min(20);
                let backoff = options
                    .connect_backoff_base
                    .saturating_mul(1u32 << shift)
                    .min(options.max_backoff);
                warn!(
                    "connect attempt {} to {} failed ({}); retrying in {:?}",
                    attempt, endpoint, err, backoff
                );
                tokio::time::sleep(backoff).await;
            }
        }
    }
}

async fn start_async(
    render_context: RenderContext,
    endpoint: String,
    catalog_path: String,
    options: ClientOptions,
) -> anyhow::Result<()> {
    let mut client = connect_with_retry(endpoint, &options).await?;

    let (client_tx, client_rx) = tokio::sync::mpsc::channel(10);
    let (script_tx, script_rx) = tokio::sync::mpsc::channel(10);
    let stream = tokio_stream::wrappers::ReceiverStream::new(client_rx);
    let request_stream = tonic::Request::new(stream);

    let client_handle = AsyncClientIoHandle::from_channels(client_tx.clone(), script_rx);

    let mut response_stream = client.streaming_api(request_stream).await?.into_inner();

    // Spawn terminal client handler in a blocking thread
    let terminal_client = TerminalClient::new(client_handle);
    let handle = tokio::task::spawn_blocking(move || {
        terminal_client.run();
        debug!("Disconnecting from server");
    });

    // Send Initialize message
    let initialize = create_initialize_message(render_context, catalog_path);
    client_tx.send(initialize).await?;

    // Forward ScriptMessages from gRPC stream to the client handle
    while let Some(script_message) = response_stream.message().await? {
        match &script_message.message {
            Some(Message::CompleteSuccess(_)) | Some(Message::CompleteError(_)) => {
                script_tx.send(script_message).await?;
                break;
            }
            _ => {
                script_tx.send(script_message).await?;
            }
        }
    }

    handle.await?;
    Ok(())
}

fn build_client_tls_config(tls: &ClientTlsOptions) -> anyhow::Result<ClientTlsConfig> {
    let mut config = ClientTlsConfig::new().with_enabled_roots();

    if let Some(ca_path) = &tls.ca_cert_path {
        let ca_pem = fs::read(ca_path).map_err(|err| {
            anyhow::anyhow!("Failed to read CA cert '{}': {}", ca_path.display(), err)
        })?;
        config = config.ca_certificate(Certificate::from_pem(ca_pem));
    }

    if let (Some(cert_path), Some(key_path)) = (&tls.client_cert_path, &tls.client_key_path) {
        let cert_pem = fs::read(cert_path).map_err(|err| {
            anyhow::anyhow!(
                "Failed to read client cert '{}': {}",
                cert_path.display(),
                err
            )
        })?;
        let key_pem = fs::read(key_path).map_err(|err| {
            anyhow::anyhow!(
                "Failed to read client key '{}': {}",
                key_path.display(),
                err
            )
        })?;
        config = config.identity(Identity::from_pem(cert_pem, key_pem));
    } else if tls.client_cert_path.is_some() || tls.client_key_path.is_some() {
        return Err(anyhow::anyhow!(
            "Client mTLS requires BOTH --tls-client-cert and --tls-client-key"
        ));
    }

    if let Some(domain) = &tls.domain_name {
        config = config.domain_name(domain.clone());
    }

    Ok(config)
}

fn create_initialize_message(value: RenderContext, catalog_path: String) -> grpc::ClientMessage {
    let api_message = ClientMessage::Initialize {
        answers_yaml: serde_yaml::to_string(value.answers()).unwrap_or_default(),
        switches: value.switches().iter().map(|v| v.to_string()).collect(),
        use_defaults: value.use_defaults().iter().map(|v| v.to_string()).collect(),
        use_defaults_all: value.use_defaults_all(),
        destination: value.destination().to_string(),
        catalog_path,
    };
    api_message.into()
}
