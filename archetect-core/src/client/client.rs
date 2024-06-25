use tracing::debug;

use archetect_api::ClientMessage;
use archetect_terminal_io::TerminalClient;

use crate::archetype::render_context::RenderContext;
use crate::io::AsyncClientIoHandle;
use crate::proto;
use crate::proto::archetect_service_client::ArchetectServiceClient;
use crate::proto::script_message::Message;
use crate::proto::ScriptMessage;

pub async fn start(render_context: RenderContext, endpoint: String) -> anyhow::Result<()> {
    let mut client = ArchetectServiceClient::connect(endpoint).await?;

    let (client_tx, client_rx) = tokio::sync::mpsc::channel(10);
    let (script_tx, script_rx) = tokio::sync::mpsc::channel(10);
    let stream = tokio_stream::wrappers::ReceiverStream::new(client_rx);
    let request_stream = tonic::Request::new(stream);

    let client_handle = AsyncClientIoHandle::from_channels(client_tx.clone(), script_rx);
    let terminal_client = TerminalClient::new(client_handle);

    let mut response_stream = client.streaming_api(request_stream).await?.into_inner();

    let handle = tokio::task::spawn_blocking(move || {
        while let Ok(()) = terminal_client.receive_script_message() {
            // Working as expected
        }
        debug!("Disconnecting from Server");
    });

    // Initialize
    let initialize = create_initialize_message(render_context);
    client_tx.send(initialize.into()).await?;

    // Process each ScriptMessage by sending it into the Terminal Client
    while let Some(script_message) = response_stream.message().await? {
        match script_message {
            // If we receive a CompleteSuccess from the server, pass it on to the TerminalClient so that it will stop
            // the client loop, and then exit the client interaction
            ScriptMessage {
                message: Some(Message::CompleteSuccess(_success)),
            } => {
                script_tx
                    .send(ScriptMessage {
                        message: Some(Message::CompleteSuccess(proto::CompleteSuccess {})),
                    })
                    .await?;
                break;
            }
            // If we receive a CompleteError from the server, pass it on to the TerminalClient so that it will
            // output the error message and stop the client loop
            ScriptMessage {
                message: Some(Message::CompleteError(error)),
            } => {
                script_tx
                    .send(ScriptMessage {
                        message: Some(Message::CompleteError(proto::CompleteError { message: error.message })),
                    })
                    .await?;
                break;
            }
            script_message => script_tx.send(script_message).await?,
        }
    }
    handle.await?;
    Ok(())
}

fn create_initialize_message(value: RenderContext) -> ClientMessage {
    ClientMessage::Initialize {
        answers_yaml: serde_yaml::to_string(value.answers()).unwrap(),
        switches: value.switches().iter().map(|v| v.to_string()).collect(),
        use_defaults: value.use_defaults().iter().map(|v| v.to_string()).collect(),
        use_defaults_all: value.use_defaults_all(),
        destination: value.destination().to_string(),
    }
}
