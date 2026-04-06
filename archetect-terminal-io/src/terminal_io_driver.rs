use std::sync::mpsc::{Receiver, SyncSender};
use std::sync::{mpsc, Arc, Mutex};

use log::{debug, error, info, trace, warn};

use crate::list_prompt_handler::handle_list_prompt;
use archetect_api::{ScriptMessage, ClientMessage, IoError, ScriptIoHandle};

use crate::bool_prompt_handler::handle_prompt_bool;
use crate::editor_prompt_info::handle_editor_prompt;
use crate::int_prompt_handler::handle_prompt_int;
use crate::multiselect_prompt_handler::handle_multiselect_prompt;
use crate::select_prompt_handler::handle_select_prompt;
use crate::text_prompt_handler::handle_prompt_text;
use crate::write_directory_handler::handle_write_directory;
use crate::write_file_handler::handle_write_file;

#[derive(Clone, Debug)]
pub struct TerminalScriptIoHandle {
    responses_tx: SyncSender<ClientMessage>,
    responses_rx: Arc<Mutex<Receiver<ClientMessage>>>,
}

impl ScriptIoHandle for TerminalScriptIoHandle {
    fn send(&self, request: ScriptMessage) -> Result<(), IoError> {
        match request {
            ScriptMessage::PromptForText(prompt_info) => {
                handle_prompt_text(prompt_info, &self.responses_tx);
            }
            ScriptMessage::PromptForInt(prompt_info) => {
                handle_prompt_int(prompt_info, &self.responses_tx);
            }
            ScriptMessage::PromptForBool(prompt_info) => {
                handle_prompt_bool(prompt_info, &self.responses_tx);
            }
            ScriptMessage::PromptForList(prompt_info) => {
                handle_list_prompt(prompt_info, &self.responses_tx);
            }
            ScriptMessage::PromptForSelect(prompt_info) => {
                handle_select_prompt(prompt_info, &self.responses_tx);
            }
            ScriptMessage::PromptForMultiSelect(prompt_info) => {
                handle_multiselect_prompt(prompt_info, &self.responses_tx);
            }
            ScriptMessage::PromptForEditor(prompt_info) => {
                handle_editor_prompt(prompt_info, &self.responses_tx);
            }
            ScriptMessage::LogInfo(message) => {
                info!("{}", message)
            }
            ScriptMessage::LogWarn(message) => {
                warn!("{}", message)
            }
            ScriptMessage::LogDebug(message) => {
                debug!("{}", message)
            }
            ScriptMessage::LogTrace(message) => {
                trace!("{}", message)
            }
            ScriptMessage::LogError(message) => {
                error!("{}", message)
            }
            ScriptMessage::Print(message) => {
                println!("{}", message)
            }
            ScriptMessage::Display(message) => {
                eprintln!("{}", message)
            }
            ScriptMessage::WriteFile(write_info) => {
                handle_write_file(write_info, &self.responses_tx);
            }
            ScriptMessage::WriteDirectory(write_info) => {
                handle_write_directory(write_info, &self.responses_tx);
            }
            ScriptMessage::CompleteSuccess => {
                debug!("Archetype completed successfully");
            }
            ScriptMessage::CompleteError(message) => {
                error!("Archetype completed with error: {}", message);
            }
        }
        Ok(())
    }

    fn receive(&self) -> Result<ClientMessage, IoError> {
        self.responses_rx
            .lock()
            .expect("Lock Error")
            .recv()
            .map_err(|_| IoError::ClientDisconnected)
    }
}

impl Default for TerminalScriptIoHandle {
    fn default() -> Self {
        let (responses_tx, responses_rx) = mpsc::sync_channel(1);
        Self {
            responses_tx,
            responses_rx: Arc::new(Mutex::new(responses_rx)),
        }
    }
}
