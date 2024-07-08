use archetect_api::{
    ClientMessage, ScriptIoHandle, ScriptMessage, SyncClientIoHandle, SyncIoDriver, SyncScriptIoHandle,
};

use crate::TerminalClient;

#[derive(Clone, Debug)]
pub struct TerminalIoDriver<SIO, CIO> {
    script_handle: SIO,
    terminal_client: TerminalClient<CIO>,
}

impl ScriptIoHandle for TerminalIoDriver<SyncScriptIoHandle, SyncClientIoHandle> {
    fn send(&self, request: ScriptMessage) -> Option<()> {
        self.script_handle.send(request);
        self.terminal_client.receive_script_message().ok()
    }

    fn receive(&self) -> Option<ClientMessage> {
        self.script_handle.receive()
    }
}

impl Default for TerminalIoDriver<SyncScriptIoHandle, SyncClientIoHandle> {
    fn default() -> Self {
        let (script_handle, client_handle) = SyncIoDriver::new().split();
        Self {
            script_handle,
            terminal_client: TerminalClient::new(client_handle),
        }
    }
}
