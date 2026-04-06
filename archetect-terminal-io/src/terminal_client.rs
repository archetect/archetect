use log::{debug, error, info, trace, warn};

use archetect_api::{ClientIoHandle, ScriptMessage};

use crate::bool_prompt_handler::handle_prompt_bool;
use crate::editor_prompt_info::handle_editor_prompt;
use crate::int_prompt_handler::handle_prompt_int;
use crate::list_prompt_handler::handle_list_prompt;
use crate::multiselect_prompt_handler::handle_multiselect_prompt;
use crate::responder::Responder;
use crate::select_prompt_handler::handle_select_prompt;
use crate::text_prompt_handler::handle_prompt_text;
use crate::write_directory_handler::handle_write_directory;
use crate::write_file_handler::handle_write_file;

pub struct TerminalClient<IO> {
    client_handle: IO,
}

impl<IO> TerminalClient<IO>
where
    IO: ClientIoHandle,
{
    pub fn new(client_handle: IO) -> Self {
        Self { client_handle }
    }

    pub fn run(&self) {
        loop {
            match self.client_handle.receive() {
                Ok(script_message) => {
                    if !self.handle_message(script_message) {
                        break;
                    }
                }
                Err(_) => {
                    debug!("Script channel closed");
                    break;
                }
            }
        }
    }

    fn handle_message(&self, message: ScriptMessage) -> bool {
        let responder = ClientIoResponder(&self.client_handle);
        match message {
            ScriptMessage::PromptForText(info) => handle_prompt_text(info, &responder),
            ScriptMessage::PromptForInt(info) => handle_prompt_int(info, &responder),
            ScriptMessage::PromptForBool(info) => handle_prompt_bool(info, &responder),
            ScriptMessage::PromptForList(info) => handle_list_prompt(info, &responder),
            ScriptMessage::PromptForSelect(info) => handle_select_prompt(info, &responder),
            ScriptMessage::PromptForMultiSelect(info) => {
                handle_multiselect_prompt(info, &responder)
            }
            ScriptMessage::PromptForEditor(info) => handle_editor_prompt(info, &responder),
            ScriptMessage::LogInfo(msg) => info!("{}", msg),
            ScriptMessage::LogWarn(msg) => warn!("{}", msg),
            ScriptMessage::LogDebug(msg) => debug!("{}", msg),
            ScriptMessage::LogTrace(msg) => trace!("{}", msg),
            ScriptMessage::LogError(msg) => error!("{}", msg),
            ScriptMessage::Print(msg) => println!("{}", msg),
            ScriptMessage::Display(msg) => eprintln!("{}", msg),
            ScriptMessage::WriteFile(info) => handle_write_file(info, &responder),
            ScriptMessage::WriteDirectory(info) => handle_write_directory(info, &responder),
            ScriptMessage::CompleteSuccess => {
                debug!("Archetype completed successfully");
                return false;
            }
            ScriptMessage::CompleteError(msg) => {
                error!("Archetype completed with error: {}", msg);
                return false;
            }
        }
        true
    }
}

struct ClientIoResponder<'a, IO: ClientIoHandle>(&'a IO);

impl<'a, IO: ClientIoHandle> Responder for ClientIoResponder<'a, IO> {
    fn respond(&self, message: archetect_api::ClientMessage) {
        let _ = self.0.send(message);
    }
}
