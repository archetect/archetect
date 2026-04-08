use std::pin::Pin;
use std::time::Duration;

use archetect_api::ContextMap;
use tokio::sync::mpsc;
use tokio::time::sleep;
use tokio_stream::{Stream, StreamExt};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status, Streaming};
use tracing::{error, info, warn};

use archetect_api::ScriptMessage;

use crate::archetype::render_context::RenderContext;
use crate::errors::ArchetectError;
use crate::io::AsyncScriptIoHandle;
use crate::proto::grpc;
use crate::proto::grpc::archetect_service_server::ArchetectService;
use crate::Archetect;

#[derive(Clone, Debug)]
pub struct ArchetectServiceCore {
    prototype: Archetect,
}

impl ArchetectServiceCore {
    pub fn builder(prototype: Archetect) -> ArchetectServiceCoreBuilder {
        ArchetectServiceCoreBuilder::new(prototype)
    }

    pub fn prototype(&self) -> &Archetect {
        &self.prototype
    }
}

pub struct ArchetectServiceCoreBuilder {
    prototype: Archetect,
}

impl ArchetectServiceCoreBuilder {
    pub fn new(prototype: Archetect) -> Self {
        Self { prototype }
    }

    pub async fn build(self) -> Result<ArchetectServiceCore, ArchetectError> {
        Ok(ArchetectServiceCore {
            prototype: self.prototype,
        })
    }
}

type ResponseStream =
    Pin<Box<dyn Stream<Item = Result<grpc::ScriptMessage, Status>> + Send>>;

#[tonic::async_trait]
impl ArchetectService for ArchetectServiceCore {
    type StreamingApiStream = ResponseStream;

    async fn streaming_api(
        &self,
        request: Request<Streaming<grpc::ClientMessage>>,
    ) -> Result<Response<Self::StreamingApiStream>, Status> {
        info!("Archetect Bidirectional Streaming API Initiating");

        let mut in_stream = request.into_inner();

        let (client_tx, client_rx) = mpsc::channel(10);
        let (script_tx, script_rx) = mpsc::channel(10);
        let client_failure_tx = client_tx.clone();

        let script_handle = AsyncScriptIoHandle::from_channels(script_tx, client_rx);
        let archetect = Archetect::builder()
            .with_configuration(self.prototype().configuration().clone())
            .with_driver(script_handle)
            .build()
            .map_err(|e| Status::internal(format!("Failed to initialize Archetect: {}", e)))?;

        let mut archetect_handle = None;
        let mut initialized = false;

        tokio::spawn(async move {
            while let Some(message) = in_stream.next().await {
                match message {
                    Ok(message) => {
                        if !initialized {
                            let archetect = archetect.clone();
                            archetect_handle = Some(tokio::task::spawn_blocking(move || {
                                if let grpc::ClientMessage {
                                    message:
                                        Some(grpc::client_message::Message::Initialize(initialize)),
                                } = message
                                {
                                    let answers = serde_yaml::from_str::<ContextMap>(
                                        &initialize.answers_yaml,
                                    )
                                    .unwrap_or_else(|err| {
                                        warn!("Failed to parse answers YAML: {}", err);
                                        ContextMap::new()
                                    });

                                    let destination = initialize.destination;
                                    let source = archetect
                                        .configuration()
                                        .action("default")
                                        .and_then(|action| match action {
                                            crate::actions::ArchetectAction::RenderArchetype {
                                                info,
                                                ..
                                            } => Some(info.source().to_string()),
                                            _ => None,
                                        });

                                    if let Some(source) = source {
                                        let render_context = RenderContext::new(destination, answers)
                                            .with_switches(
                                                initialize
                                                    .switches
                                                    .iter()
                                                    .map(|v| v.to_string())
                                                    .collect(),
                                            )
                                            .with_use_defaults(
                                                initialize
                                                    .use_defaults
                                                    .iter()
                                                    .map(|v| v.to_string())
                                                    .collect(),
                                            )
                                            .with_use_defaults_all(initialize.use_defaults_all);

                                        match archetect.new_archetype(&source) {
                                            Ok(archetype) => {
                                                match archetype.render(render_context) {
                                                    Ok(_) => {
                                                        info!("Successfully rendered");
                                                        let _ = archetect.request(
                                                            ScriptMessage::CompleteSuccess,
                                                        );
                                                    }
                                                    Err(err) => {
                                                        error!("Render error: {:?}", err);
                                                    }
                                                }
                                            }
                                            Err(err) => {
                                                let _ = archetect.request(
                                                    ScriptMessage::CompleteError(err.to_string()),
                                                );
                                            }
                                        }
                                    } else {
                                        let _ = archetect.request(ScriptMessage::CompleteError(
                                            "No default action configured".to_string(),
                                        ));
                                    }
                                } else {
                                    let _ = archetect.request(ScriptMessage::LogError(
                                        "Improper Initialization Message".to_string(),
                                    ));
                                }
                            }));

                            initialized = true;
                        } else {
                            let _ = client_tx.send(message).await;
                        }
                    }
                    Err(err) => {
                        warn!("gRPC Error: {}. Sending Abort Message", err);
                        let _ = client_failure_tx
                            .send(grpc::ClientMessage {
                                message: Some(grpc::client_message::Message::Abort(())),
                            })
                            .await;
                    }
                }
            }

            if let Some(handle) = archetect_handle {
                tokio::select! {
                    _ = handle => {
                        info!("Archetect thread closed successfully");
                    },
                    _ = sleep(Duration::from_secs(30)) => {
                        error!("Archetect thread failed to close within 30 seconds");
                    }
                };
            }
            info!("Client disconnected");
        });

        let out_stream = ReceiverStream::new(script_rx).map(Ok);

        Ok(Response::new(
            Box::pin(out_stream) as Self::StreamingApiStream
        ))
    }
}
