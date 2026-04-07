use std::fmt;
use std::sync::{Arc, Mutex};

use tokio::sync::mpsc::{Receiver, Sender};

use archetect_api::{ClientMessage, IoError, ScriptIoHandle, ScriptMessage};

/// A ScriptIoHandle implementation that bridges the blocking render thread
/// with async MCP tool handlers via tokio mpsc channels.
pub struct McpScriptIoHandle {
    script_tx: Sender<ScriptMessage>,
    client_rx: Arc<Mutex<Receiver<ClientMessage>>>,
}

impl McpScriptIoHandle {
    pub fn new(
        script_tx: Sender<ScriptMessage>,
        client_rx: Receiver<ClientMessage>,
    ) -> Self {
        Self {
            script_tx,
            client_rx: Arc::new(Mutex::new(client_rx)),
        }
    }
}

impl Clone for McpScriptIoHandle {
    fn clone(&self) -> Self {
        Self {
            script_tx: self.script_tx.clone(),
            client_rx: self.client_rx.clone(),
        }
    }
}

impl fmt::Debug for McpScriptIoHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("McpScriptIoHandle").finish()
    }
}

impl ScriptIoHandle for McpScriptIoHandle {
    fn send(&self, request: ScriptMessage) -> Result<(), IoError> {
        self.script_tx
            .blocking_send(request)
            .map_err(|_| IoError::ClientDisconnected)
    }

    fn receive(&self) -> Result<ClientMessage, IoError> {
        self.client_rx
            .lock()
            .expect("Lock Error")
            .blocking_recv()
            .ok_or(IoError::ClientDisconnected)
    }
}
