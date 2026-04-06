use tracing::debug;

use archetect_api::ClientMessage;
use archetect_terminal_io::TerminalClient;

use crate::archetype::render_context::RenderContext;
use crate::errors::ArchetectError;
use crate::io::AsyncClientIoHandle;
use crate::proto::grpc;
use crate::proto::grpc::archetect_service_client::ArchetectServiceClient;
use crate::proto::grpc::script_message::Message;

pub fn start(render_context: RenderContext, endpoint: String) -> Result<(), ArchetectError> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Viable Tokio Runtime");

    runtime
        .block_on(start_async(render_context, endpoint))
        .map_err(|err: anyhow::Error| ArchetectError::ServerError(format!("Client connection error: {}", err)))?;

    Ok(())
}

async fn start_async(
    render_context: RenderContext,
    endpoint: String,
) -> anyhow::Result<()> {
    let mut client = ArchetectServiceClient::connect(endpoint).await?;

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
    let initialize = create_initialize_message(render_context);
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

fn create_initialize_message(value: RenderContext) -> grpc::ClientMessage {
    let api_message = ClientMessage::Initialize {
        answers_yaml: serde_yaml::to_string(value.answers()).unwrap_or_default(),
        switches: value.switches().iter().map(|v| v.to_string()).collect(),
        use_defaults: value.use_defaults().iter().map(|v| v.to_string()).collect(),
        use_defaults_all: value.use_defaults_all(),
        destination: value.destination().to_string(),
    };
    api_message.into()
}
