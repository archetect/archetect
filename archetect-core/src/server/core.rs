use std::error::Error;
use std::io::ErrorKind;
use std::pin::Pin;

use rhai::Map;
use tokio::sync::mpsc;
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
    pub fn builder(prototype: Archetect) -> Builder {
        Builder::new(prototype)
    }

    pub fn prototype(&self) -> &Archetect {
        &self.prototype
    }
}

pub struct Builder {
    prototype: Archetect,
}

impl Builder {
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
                                if let Some(banner) = archetect.configuration().server().banner() {
                                    archetect.request(ScriptMessage::Display(banner.to_string()));
                                }
                                if let ClientMessage {
                                    message: Some(Message::Initialize(initialize)),
                                } = message
                                {
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
                                            archetect.request(ScriptMessage::CompleteSuccess);
                                        }
                                        Err(error) => {
                                            error!("Exited with Error: \n{:?}", error);
                                            archetect.request(ScriptMessage::CompleteError {
                                                message: error.to_string(),
                                            });
                                            return;
                                        }
                                    }
                                } else {
                                    archetect.request(ScriptMessage::LogError(
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
                        if let Some(io_err) = match_for_io_error(&err) {
                            if io_err.kind() == ErrorKind::BrokenPipe {
                                // here you can handle special case when client
                                // disconnected in unexpected way
                                warn!("\tclient disconnected: broken pipe");
                                break;
                            }
                        }
                    }
                }
            }
            info!("Client Disconnected");
        });

        // echo just write the same data that was received
        let out_stream = ReceiverStream::new(script_rx).map(|message| Ok(message));

        Ok(Response::new(Box::pin(out_stream) as Self::StreamingApiStream))
    }
}

type ResponseStream = Pin<Box<dyn Stream<Item = Result<proto::ScriptMessage, Status>> + Send>>;

fn match_for_io_error(err_status: &Status) -> Option<&std::io::Error> {
    let mut err: &(dyn Error + 'static) = err_status;

    loop {
        if let Some(io_err) = err.downcast_ref::<std::io::Error>() {
            return Some(io_err);
        }

        // h2::Error do not expose std::io::Error with `source()`
        // https://github.com/hyperium/h2/pull/462
        if let Some(h2_err) = err.downcast_ref::<h2::Error>() {
            if let Some(io_err) = h2_err.get_io() {
                return Some(io_err);
            }
        }

        err = match err.source() {
            Some(err) => err,
            None => return None,
        };
    }
}
