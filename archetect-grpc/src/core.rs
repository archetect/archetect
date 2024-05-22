use std::error::Error;
use std::io::ErrorKind;
use std::pin::Pin;

use anyhow::Result;
use tokio::sync::mpsc;
use tokio_stream::{Stream, StreamExt};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status, Streaming};

use archetect_core::Archetect;

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

    pub async fn build(self) -> Result<ArchetectServiceCore> {
        Ok(ArchetectServiceCore {})
    }
}

#[tonic::async_trait]
impl ArchetectService for ArchetectServiceCore {
    type StreamingApiStream = ResponseStream;

    async fn streaming_api(&self, request: Request<Streaming<ClientMessage>>) -> std::result::Result<Response<Self::StreamingApiStream>, Status> {
        println!("Archetect Bidirectional Streaming API Initiating");

        let mut in_stream = request.into_inner();

        // this spawn here is required if you want to handle connection error.
        // If we just map `in_stream` and write it back as `out_stream` the `out_stream`
        // will be drooped when connection error occurs and error will never be propagated
        // to mapped version of `in_stream`.

        // let (client_message_tx, client_message_rx) = mpsc::channel(1);
        let (script_message_tx, script_message_rx) = mpsc::channel(128);
        // let client_message_rx = Arc::new(Mutex::new(client_message_rx));

        let archetect = Archetect::builder().build().expect("Unable to bootstrap Archetect :(");
        let mut archetect_handle = None;
        let mut initialized = false;
        tokio::spawn(async move {
            while let Some(message) = in_stream.next().await {
                let script_message_tx_clone = script_message_tx.clone();
                // let client_message_rx_clone = client_message_rx.clone();
                match message {
                    Ok(_message) => {
                        script_message_tx_clone.send(Ok(proto::ScriptMessage::default()))
                            .await.expect("Failure to send message :(");

                        if !initialized {
                            let clone = archetect.clone();
                            archetect_handle = Some(
                                tokio::task::spawn_blocking(move || {
                                    let clone = clone.clone();
                                }));

                            initialized = true;
                        }

                        // if !bootstrapped {
                        //     println!("Initializing Archetect for this client");
                        //     let handle = tokio::task::spawn_blocking(move || {
                        //         let archetect = Archetect {
                        //             driver: GrpcScriptIoDriver {
                        //                 // ScriptMessage protobufs
                        //                 script_message_tx: script_message_tx_clone,
                        //                 // ClientMessage protobufs
                        //                 client_message_rx: client_message_rx_clone,
                        //             }.into(),
                        //         };
                        //         archetect.execute();
                        //     });
                        //     bootstrapped = true;
                        // } else {
                        //     client_message_tx.send(message).await.expect("Error \
                        //     Sending Client Message");
                        // }
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
        let out_stream = ReceiverStream::new(script_message_rx);

        Ok(Response::new(
            Box::pin(out_stream) as Self::StreamingApiStream
        ))
    }
}

type ResponseStream = Pin<Box<dyn Stream<Item=std::result::Result<proto::ScriptMessage, Status>> +
Send>>;

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
