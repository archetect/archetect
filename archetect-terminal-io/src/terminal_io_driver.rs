use std::sync::mpsc::{Receiver, SyncSender};
use std::sync::{mpsc, Arc, Mutex};

use log::{debug, error, info, trace, warn};

use crate::list_prompt_handler::handle_list_prompt;
use archetect_api::{CommandRequest, CommandResponse, IoDriver};

use crate::bool_prompt_handler::handle_prompt_bool;
use crate::int_prompt_handler::handle_prompt_int;
use crate::multiselect_prompt_handler::handle_multiselect_prompt;
use crate::select_prompt_handler::handle_select_prompt;
use crate::text_prompt_handler::handle_prompt_text;

#[derive(Clone, Debug)]
pub struct TerminalIoDriver {
    responses_tx: SyncSender<CommandResponse>,
    responses_rx: Arc<Mutex<Receiver<CommandResponse>>>,
}

impl IoDriver for TerminalIoDriver {
    fn send(&self, request: CommandRequest) {
        match request {
            CommandRequest::PromptForText(prompt_info) => {
                handle_prompt_text(prompt_info, &self.responses_tx);
            }
            CommandRequest::PromptForInt(prompt_info) => {
                handle_prompt_int(prompt_info, &self.responses_tx);
            }
            CommandRequest::PromptForBool(prompt_info) => {
                handle_prompt_bool(prompt_info, &self.responses_tx);
            }
            CommandRequest::PromptForList(prompt_info) => {
                handle_list_prompt(prompt_info, &self.responses_tx);
            }
            CommandRequest::PromptForSelect(prompt_info) => {
                handle_select_prompt(prompt_info, &self.responses_tx);
            }
            CommandRequest::PromptForMultiSelect(prompt_info) => {
                handle_multiselect_prompt(prompt_info, &self.responses_tx);
            }
            CommandRequest::LogInfo(message) => {
                info!("{}", message)
            }
            CommandRequest::LogWarn(message) => {
                warn!("{}", message)
            }
            CommandRequest::LogDebug(message) => {
                debug!("{}", message)
            }
            CommandRequest::LogTrace(message) => {
                trace!("{}", message)
            }
            CommandRequest::LogError(message) => {
                error!("{}", message)
            }
            CommandRequest::Print(message) => {
                println!("{}", message)
            }
            CommandRequest::Display(message) => {
                eprintln!("{}", message)
            }
        }
    }

    fn responses(&self) -> Arc<Mutex<Receiver<CommandResponse>>> {
        self.responses_rx.clone()
    }
}

impl Default for TerminalIoDriver {
    fn default() -> Self {
        let (responses_tx, responses_rx) = mpsc::sync_channel(1);
        Self {
            responses_tx,
            responses_rx: Arc::new(Mutex::new(responses_rx)),
        }
    }
}
