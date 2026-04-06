use std::fmt::Debug;
use std::sync::mpsc::{Receiver, SyncSender};
use std::sync::{mpsc, Arc, Mutex};

use crate::{ClientMessage, IoError, ScriptMessage};

pub trait ScriptIoHandle: Debug + Send + Sync + 'static {
    fn send(&self, request: ScriptMessage) -> Result<(), IoError>;

    fn receive(&self) -> Result<ClientMessage, IoError>;
}

impl<T: ScriptIoHandle> From<T> for Box<dyn ScriptIoHandle> {
    fn from(value: T) -> Self {
        Box::new(value)
    }
}

pub trait ClientIoHandle: Debug + Send + Sync + 'static {
    fn send(&self, message: ClientMessage) -> Result<(), IoError>;

    fn receive(&self) -> Result<ScriptMessage, IoError>;
}

impl<T: ClientIoHandle> From<T> for Box<dyn ClientIoHandle> {
    fn from(value: T) -> Self {
        Box::new(value)
    }
}

pub fn sync_io_channel() -> (SyncScriptIoHandle, SyncClientIoHandle) {
    let (requests_tx, requests_rx) = mpsc::sync_channel(1);
    let (responses_tx, responses_rx) = mpsc::sync_channel(1);
    let script_handle = SyncScriptIoHandle {
        requests_tx,
        responses_rx: Arc::new(Mutex::new(responses_rx)),
    };
    let client_handle = SyncClientIoHandle {
        responses_tx,
        requests_rx: Arc::new(Mutex::new(requests_rx)),
    };
    (script_handle, client_handle)
}

#[derive(Debug)]
pub struct SyncScriptIoHandle {
    requests_tx: SyncSender<ScriptMessage>,
    responses_rx: Arc<Mutex<Receiver<ClientMessage>>>,
}

impl ScriptIoHandle for SyncScriptIoHandle {
    fn send(&self, request: ScriptMessage) -> Result<(), IoError> {
        self.requests_tx
            .send(request)
            .map_err(|_| IoError::ClientDisconnected)
    }

    fn receive(&self) -> Result<ClientMessage, IoError> {
        self.responses_rx
            .lock()
            .expect("Lock Error")
            .recv()
            .map_err(|_| IoError::ClientDisconnected)
    }
}

#[derive(Debug)]
pub struct SyncClientIoHandle {
    requests_rx: Arc<Mutex<Receiver<ScriptMessage>>>,
    responses_tx: SyncSender<ClientMessage>,
}

impl ClientIoHandle for SyncClientIoHandle {
    fn send(&self, response: ClientMessage) -> Result<(), IoError> {
        self.responses_tx
            .send(response)
            .map_err(|_| IoError::ScriptChannelClosed)
    }

    fn receive(&self) -> Result<ScriptMessage, IoError> {
        self.requests_rx
            .lock()
            .expect("Lock Error")
            .recv()
            .map_err(|_| IoError::ScriptChannelClosed)
    }
}
