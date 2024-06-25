use dyn_clone::DynClone;
use tracing::{debug, error, info, trace, warn};

use archetect_api::{ClientIoHandle, ScriptMessage};

use crate::bool_prompt_handler::handle_prompt_bool;
use crate::editor_prompt_info::handle_editor_prompt;
use crate::int_prompt_handler::handle_prompt_int;
use crate::list_prompt_handler::handle_list_prompt;
use crate::multiselect_prompt_handler::handle_multiselect_prompt;
use crate::select_prompt_handler::handle_select_prompt;
use crate::text_prompt_handler::handle_prompt_text;
use crate::write_directory_handler::handle_write_directory;
use crate::write_file_handler::handle_write_file;

#[derive(Clone, Debug)]
pub struct TerminalClient<IO> {
    client_handle: IO,
}

impl<IO> TerminalClient<IO>
where
    IO: ClientIoHandle + Clone + DynClone + Send + Sync + 'static,
{
    pub fn new(client_handle: IO) -> Self {
        Self { client_handle }
    }

    pub fn receive_script_message(&self) -> Result<(), ()> {
        if let Some(script_message) = self.client_handle.receive() {
            match script_message {
                ScriptMessage::PromptForText(prompt_info) => {
                    handle_prompt_text(prompt_info, &self.client_handle);
                }
                ScriptMessage::PromptForInt(prompt_info) => {
                    handle_prompt_int(prompt_info, &self.client_handle);
                }
                ScriptMessage::PromptForBool(prompt_info) => {
                    handle_prompt_bool(prompt_info, &self.client_handle);
                }
                ScriptMessage::PromptForList(prompt_info) => {
                    handle_list_prompt(prompt_info, &self.client_handle);
                }
                ScriptMessage::PromptForSelect(prompt_info) => {
                    handle_select_prompt(prompt_info, &self.client_handle);
                }
                ScriptMessage::PromptForMultiSelect(prompt_info) => {
                    handle_multiselect_prompt(prompt_info, &self.client_handle);
                }
                ScriptMessage::PromptForEditor(prompt_info) => {
                    handle_editor_prompt(prompt_info, &self.client_handle);
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
                ScriptMessage::CompleteSuccess => return Err(()),
                ScriptMessage::CompleteError { message } => {
                    error!("{}", message);
                    return Err(());
                }
                ScriptMessage::WriteFile(write_info) => handle_write_file(write_info, &self.client_handle),
                ScriptMessage::WriteDirectory(write_info) => {
                    handle_write_directory(write_info, &self.client_handle);
                }
            }
            Ok(())
        } else {
            warn!("No Script Message Available!");
            Err(())
        }
    }
}
