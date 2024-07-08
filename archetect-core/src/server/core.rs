use std::pin::Pin;
use std::time::Duration;

use rhai::Map;
use tokio::sync::mpsc;
use tokio::time::sleep;
use tokio_stream::{Stream, StreamExt};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status, Streaming};
use tracing::{error, info, warn};

use archetect_api::ScriptMessage;

use crate::{Archetect, proto};
use crate::archetype::render_context::RenderContext;
use crate::errors::ArchetectError;
use crate::io::AsyncScriptIoHandle;
use crate::proto::archetect_service_server::ArchetectService;
use crate::proto::client_message::Message;
use crate::proto::ClientMessage;

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

#[tonic::async_trait]
impl ArchetectService for ArchetectServiceCore {
    type StreamingApiStream = ResponseStream;

    async fn streaming_api(
        &self,
        request: Request<Streaming<ClientMessage>>,
    ) -> Result<Response<Self::StreamingApiStream>, Status> {
        info!("Archetect Bidirectional Streaming API Initiating");

        let mut in_stream = request.into_inner();

        // this spawn here is required if you want to handle connection error.
        // If we just map `in_stream` and write it back as `out_stream` the `out_stream`
        // will be drooped when connection error occurs and error will never be propagated
        // to mapped version of `in_stream`.

        let (client_tx, client_rx) = mpsc::channel(10);
        let (script_tx, script_rx) = mpsc::channel(10);
        let client_failure_tx = client_tx.clone();

        let script_handle = AsyncScriptIoHandle::from_channels(script_tx, client_rx);
        let archetect = Archetect::builder()
            .with_configuration(self.prototype().configuration().clone())
            .with_driver(script_handle)
            .build()
            .expect("Unable to bootstrap Archetect :(");

        let mut archetect_handle = None;
        let mut initialized = false;
        tokio::spawn(async move {
            while let Some(message) = in_stream.next().await {
                match message {
                    Ok(message) => {
                        if !initialized {
                            let archetect = archetect.clone();
                            archetect_handle = Some(tokio::task::spawn_blocking(move || {
                                if let ClientMessage {
                                    message: Some(Message::Initialize(initialize)),
                                } = message
                                {
                                    if let Some(banner) = archetect.configuration().server().banner() {
                                        let _ = archetect.request(ScriptMessage::Display(banner.to_string()));
                                    }
                                    let answers = serde_yaml::from_str::<Map>(&initialize.answers_yaml).unwrap();
                                    let render_context = RenderContext::new(initialize.destination, answers)
                                        .with_switches(initialize.switches.iter().map(|v| v.to_string()).collect())
                                        .with_use_defaults(
                                            initialize.use_defaults.iter().map(|v| v.to_string()).collect(),
                                        )
                                        .with_use_defaults_all(initialize.use_defaults_all);

                                    match archetect.execute_action("default", render_context) {
                                        Ok(_success) => {
                                            info!("Successfully Rendered... Sending Disconnect");
                                            let _ = archetect.request(ScriptMessage::CompleteSuccess);
                                        }
                                        Err(error) => {
                                            error!("Exited with Error: \n{:?}", error);
                                            let _ = archetect.request(ScriptMessage::CompleteError {
                                                message: error.to_string(),
                                            });
                                            return;
                                        }
                                    }
                                } else {
                                    let _ = archetect.request(ScriptMessage::LogError(
                                        "Improper Initialization Message".to_string(),
                                    ));
                                    return;
                                }
                            }));

                            initialized = true;
                        } else {
                            let _unhandled = client_tx.send(message).await;
                        }
                    }
                    Err(err) => {
                        warn!("gRPC Error: {}. Sending Abort Message", err);
                        // Regardless of error, send an Abort message to exit from Script Execution
                        let _ = client_failure_tx
                            .send(ClientMessage {
                                message: Some(Message::Abort(())),
                            })
                            .await;
                    }
                }
            }
            if let Some(handle) = archetect_handle {
                tokio::select! {
                    _ = handle => {
                        info!("Archetect Thread Closed Successfully");
                    },
                    _ = sleep(Duration::from_secs(5)) => {
                        error!("Archetect Thread Failed to Close within 5 seconds");
                    }
                };
            } else {
                warn!("No Archetect Thread allocated")
            }
            info!("Client Disconnected");
        });

        // echo just write the same data that was received
        let out_stream = ReceiverStream::new(script_rx).map(|message| Ok(message));

        Ok(Response::new(Box::pin(out_stream) as Self::StreamingApiStream))
    }
}

type ResponseStream = Pin<Box<dyn Stream<Item = Result<proto::ScriptMessage, Status>> + Send>>;
