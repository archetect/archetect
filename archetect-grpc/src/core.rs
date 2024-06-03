use std::error::Error;
use std::io::ErrorKind;
use std::pin::Pin;

use rhai::Map;
use tokio::sync::mpsc;
use tokio_stream::{Stream, StreamExt};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status, Streaming};

use archetect_api::{ScriptMessage, TextPromptInfo};
use archetect_core::Archetect;
use archetect_core::archetype::render_context::RenderContext;
use archetect_core::errors::ArchetectError;

use crate::io::AsyncScriptIoHandle;
use crate::proto;
use crate::proto::archetect_service_server::ArchetectService;
use crate::proto::ClientMessage;

#[derive(Clone, Debug)]
pub struct ArchetectServiceCore {}

impl ArchetectServiceCore {
    pub fn builder() -> Builder {
        Builder::new()
    }
}

pub struct Builder {}

impl Builder {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn build(self) -> Result<ArchetectServiceCore, ArchetectError> {
        Ok(ArchetectServiceCore {})
    }
}

#[tonic::async_trait]
impl ArchetectService for ArchetectServiceCore {
    type StreamingApiStream = ResponseStream;

    async fn streaming_api(
        &self,
        request: Request<Streaming<ClientMessage>>,
    ) -> Result<Response<Self::StreamingApiStream>, Status> {
        println!("Archetect Bidirectional Streaming API Initiating");

        let mut in_stream = request.into_inner();

        // this spawn here is required if you want to handle connection error.
        // If we just map `in_stream` and write it back as `out_stream` the `out_stream`
        // will be drooped when connection error occurs and error will never be propagated
        // to mapped version of `in_stream`.

        let (client_tx, client_rx) = mpsc::channel(10);
        let (script_tx, script_rx) = mpsc::channel(10);

        let script_handle = AsyncScriptIoHandle::from_channels(script_tx, client_rx);
        let archetect = Archetect::builder()
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
                            // TODO: Verify and Use Initialize Message
                            let clone = archetect.clone();
                            archetect_handle = Some(tokio::task::spawn_blocking(move || {
                                let archetect_clone = clone.clone();

                                let render_context = RenderContext::new(".".to_owned(), Map::new());
                                let result = archetect_clone.execute_action("default", render_context);

                                archetect_clone.request(ScriptMessage::Display(
                                    "Hello, World from \
                                    Script!"
                                        .to_string(),
                                ));

                                archetect_clone.request(ScriptMessage::PromptForText(TextPromptInfo::new(
                                    "First Name:",
                                    None::<String>,
                                )));

                                let response = archetect_clone.receive();
                                println!("{response:?}");

                                archetect_clone.request(ScriptMessage::PromptForText(TextPromptInfo::new(
                                    "Last Name:",
                                    None::<String>,
                                )));

                                let response = archetect_clone.receive();
                                println!("{response:?}");

                                archetect_clone.request(ScriptMessage::LogInfo(
                                    "This is bad \
                                    ass!"
                                        .into(),
                                ));
                                archetect_clone.request(ScriptMessage::LogWarn(
                                    "This is bad \
                                    ass!"
                                        .into(),
                                ));
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
                                eprintln!("\tclient disconnected: broken pipe");
                                break;
                            }
                        }
                    }
                }
            }
            println!("\tstream ended");
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
