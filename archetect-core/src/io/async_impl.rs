use std::sync::{Arc, Mutex};

use tokio::sync::mpsc::{Receiver, Sender};

use archetect_api::{ClientMessage, IoError, ScriptIoHandle, ScriptMessage};

use crate::proto::grpc;

#[derive(Clone, Debug)]
pub struct AsyncScriptIoHandle {
    script_tx: Sender<grpc::ScriptMessage>,
    client_rx: Arc<Mutex<Receiver<grpc::ClientMessage>>>,
}

impl AsyncScriptIoHandle {
    pub fn from_channels(
        script_tx: Sender<grpc::ScriptMessage>,
        client_rx: Receiver<grpc::ClientMessage>,
    ) -> Self {
        Self {
            script_tx,
            client_rx: Arc::new(Mutex::new(client_rx)),
        }
    }
}

impl ScriptIoHandle for AsyncScriptIoHandle {
    fn send(&self, request: ScriptMessage) -> Result<(), IoError> {
        self.script_tx
            .blocking_send(request.into())
            .map_err(|_| IoError::ClientDisconnected)
    }

    fn receive(&self) -> Result<ClientMessage, IoError> {
        self.client_rx
            .lock()
            .expect("Lock Error")
            .blocking_recv()
            .map(|message| message.into())
            .ok_or(IoError::ClientDisconnected)
    }
}

#[derive(Clone, Debug)]
pub struct AsyncClientIoHandle {
    client_tx: Sender<grpc::ClientMessage>,
    script_rx: Arc<Mutex<Receiver<grpc::ScriptMessage>>>,
}

impl AsyncClientIoHandle {
    pub fn from_channels(
        client_tx: Sender<grpc::ClientMessage>,
        script_rx: Receiver<grpc::ScriptMessage>,
    ) -> Self {
        Self {
            client_tx,
            script_rx: Arc::new(Mutex::new(script_rx)),
        }
    }
}

impl archetect_api::ClientIoHandle for AsyncClientIoHandle {
    fn send(&self, message: ClientMessage) -> Result<(), IoError> {
        self.client_tx
            .blocking_send(message.into())
            .map_err(|_| IoError::ScriptChannelClosed)
    }

    fn receive(&self) -> Result<ScriptMessage, IoError> {
        self.script_rx
            .lock()
            .expect("Lock Error")
            .blocking_recv()
            .map(|message| message.into())
            .ok_or(IoError::ScriptChannelClosed)
    }
}
