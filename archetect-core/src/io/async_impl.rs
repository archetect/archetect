use std::sync::{Arc, Mutex};

use tokio::sync::mpsc::Receiver;
use tokio::sync::mpsc::Sender;

use archetect_api::{ClientIoHandle, ClientMessage, ScriptIoHandle, ScriptMessage};

use crate::proto;

#[derive(Clone, Debug)]
pub struct AsyncScriptIoHandle {
    script_tx: Sender<proto::ScriptMessage>,
    client_rx: Arc<Mutex<Receiver<proto::ClientMessage>>>,
}

impl AsyncScriptIoHandle {
    pub fn from_channels(script_tx: Sender<proto::ScriptMessage>, client_rx: Receiver<proto::ClientMessage>) -> Self {
        Self {
            script_tx,
            client_rx: Arc::new(Mutex::new(client_rx)),
        }
    }
}

impl ScriptIoHandle for AsyncScriptIoHandle {
    fn send(&self, request: ScriptMessage) -> Option<()> {
        self.script_tx.blocking_send(request.into()).ok()
    }

    fn receive(&self) -> Option<ClientMessage> {
        self.client_rx
            .lock()
            .expect("Working Mutex")
            .blocking_recv()
            .map(|message| message.into())
    }
}

#[derive(Clone, Debug)]
pub struct AsyncClientIoHandle {
    client_tx: Sender<proto::ClientMessage>,
    script_rx: Arc<Mutex<Receiver<proto::ScriptMessage>>>,
}

impl AsyncClientIoHandle {
    pub fn from_channels(client_tx: Sender<proto::ClientMessage>, script_rx: Receiver<proto::ScriptMessage>) -> Self {
        Self {
            client_tx,
            script_rx: Arc::new(Mutex::new(script_rx)),
        }
    }
}

impl ClientIoHandle for AsyncClientIoHandle {
    fn send(&self, message: ClientMessage) {
        self.client_tx.blocking_send(message.into()).expect("Working Channel");
    }

    fn receive(&self) -> Option<ScriptMessage> {
        self.script_rx
            .lock()
            .expect("Working Mutex")
            .blocking_recv()
            .map(|message| message.into())
    }
}
