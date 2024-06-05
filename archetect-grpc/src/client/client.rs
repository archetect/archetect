use tracing::{info, warn};

use archetect_api::ClientMessage;
use archetect_core::archetype::render_context::RenderContext;
use archetect_terminal_io::TerminalClient;

use crate::io::AsyncClientIoHandle;
use crate::proto::archetect_service_client::ArchetectServiceClient;

pub async fn start(render_context: RenderContext) -> anyhow::Result<()> {
    let mut client = ArchetectServiceClient::connect("http://localhost:8080").await?;

    let (client_tx, client_rx) = tokio::sync::mpsc::channel(10);
    let (script_tx, script_rx) = tokio::sync::mpsc::channel(10);
    let stream = tokio_stream::wrappers::ReceiverStream::new(client_rx);
    let request_stream = tonic::Request::new(stream);

    let client_handle = AsyncClientIoHandle::from_channels(client_tx.clone(), script_rx);
    let terminal_client = TerminalClient::new(client_handle);

    let mut response_stream = client.streaming_api(request_stream).await?.into_inner();

    let _handle = tokio::task::spawn_blocking(move || {
        while let Ok(()) = terminal_client.receive_script_message() {
            // Working as expected
        }
        warn!("Server Closed Connection");
    });

    println!("{render_context:?}");

    // Initialize
    let initialize = create_initialize_message(render_context);
    client_tx.send(initialize.into()).await?;

    // Process each ScriptMessage by sending it into the Terminal Client
    while let Some(script_message) = response_stream.message().await? {
        script_tx.send(script_message).await?;
    }
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

#[cfg(test)]
mod tests {
    use crate::start;

    #[tokio::test]
    async fn test_client() -> anyhow::Result<()> {
        start().await
    }
}
