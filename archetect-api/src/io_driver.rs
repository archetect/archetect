use std::fmt::Debug;
use std::sync::{Arc, mpsc, Mutex};
use std::sync::mpsc::{Receiver, SyncSender};

use dyn_clone::DynClone;
use tracing::warn;

use crate::{ClientMessage, ScriptMessage};

pub trait ScriptIoHandle: DynClone + Debug + Send + Sync + 'static {
    fn send(&self, request: ScriptMessage) -> Option<()>;

    fn receive(&self) -> Option<ClientMessage>;
}

impl<T: ScriptIoHandle> From<T> for Box<dyn ScriptIoHandle> {
    fn from(value: T) -> Self {
        Box::new(value)
    }
}

dyn_clone::clone_trait_object!(ScriptIoHandle);

pub trait ClientIoHandle: DynClone + Debug + Send + Sync + 'static {
    fn send(&self, message: ClientMessage);

    fn receive(&self) -> Option<ScriptMessage>;
}

impl<T: ClientIoHandle> From<T> for Box<dyn ClientIoHandle> {
    fn from(value: T) -> Self {
        Box::new(value)
    }
}

dyn_clone::clone_trait_object!(ClientIoHandle);

pub struct SyncIoDriver {
    script_handle: SyncScriptIoHandle,
    client_handle: SyncClientIoHandle,
}

impl SyncIoDriver {
    pub fn new() -> SyncIoDriver {
        let (script_tx, script_rx) = mpsc::sync_channel(1);
        let (client_tx, client_rx) = mpsc::sync_channel(1);
        let script_handle = SyncScriptIoHandle {
            script_tx,
            client_rx: Arc::new(Mutex::new(client_rx)),
        };
        let client_handle = SyncClientIoHandle {
            script_rx: Arc::new(Mutex::new(script_rx)),
            client_tx,
        };
        Self {
            script_handle,
            client_handle,
        }
    }

    pub fn script_handle(&self) -> &SyncScriptIoHandle {
        &self.script_handle
    }

    pub fn client_handle(&self) -> &SyncClientIoHandle {
        &self.client_handle
    }

    pub fn split(self) -> (SyncScriptIoHandle, SyncClientIoHandle) {
        (self.script_handle, self.client_handle)
    }
}

#[derive(Clone, Debug)]
pub struct SyncScriptIoHandle {
    script_tx: SyncSender<ScriptMessage>,
    client_rx: Arc<Mutex<Receiver<ClientMessage>>>,
}

impl ScriptIoHandle for SyncScriptIoHandle {
    fn send(&self, request: ScriptMessage) -> Option<()> {
        self.script_tx.send(request).ok()
    }

    fn receive(&self) -> Option<ClientMessage> {
        self.client_rx.lock().expect("Working Mutex").recv().ok()
    }
}

#[derive(Clone, Debug)]
pub struct SyncClientIoHandle {
    pub client_tx: SyncSender<ClientMessage>,
    pub script_rx: Arc<Mutex<Receiver<ScriptMessage>>>,
}

impl ClientIoHandle for SyncClientIoHandle {
    fn send(&self, response: ClientMessage) {
        self.client_tx.send(response).expect("Send Error")
    }

    fn receive(&self) -> Option<ScriptMessage> {
        match self.script_rx.lock().expect("Working Mutex").recv() {
            Ok(script_message) => Some(script_message),
            Err(err) => {
                warn!("Receive Error: {:?}", err);
                None
            }
        }
    }
}

impl<T: ClientIoHandle + 'static> ClientIoHandle for &'static T {
    fn send(&self, message: ClientMessage) {
        (*self).send(message)
    }

    fn receive(&self) -> Option<ScriptMessage> {
        (*self).receive()
    }
}
