use std::fs;

use camino::Utf8PathBuf;

use archetect_api::{ClientIoHandle, ClientMessage, ExistingFilePolicy, WriteFileInfo};
use archetect_inquire::Confirm;

pub fn handle_write_file<CIO: ClientIoHandle>(prompt_info: WriteFileInfo, client_handle: CIO) {
    let path = Utf8PathBuf::from(prompt_info.destination);
    match prompt_info.existing_file_policy {
        ExistingFilePolicy::Overwrite => {
            if !write_file(path, prompt_info.contents, &client_handle) {
                return;
            }
        }
        ExistingFilePolicy::Preserve => {
            if !write_file(path, prompt_info.contents, &client_handle) {
                return;
            }
        }
        ExistingFilePolicy::Prompt => {
            if path.exists() {
                if Confirm::new(format!("Overwrite '{}'?", path).as_str())
                    .prompt_skippable()
                    .unwrap_or_default()
                    .unwrap_or_default()
                {
                    if !write_file(path, prompt_info.contents, &client_handle) {
                        return;
                    }
                }
            }
        }
    }
    client_handle.send(ClientMessage::Ack);
}

fn write_file<CIO: ClientIoHandle>(path: Utf8PathBuf, contents: Vec<u8>, client_handle: &CIO) -> bool {
    match fs::write(path, contents) {
        Ok(_success) => true,
        Err(error) => {
            client_handle.send(ClientMessage::Error(error.to_string()));
            false
        }
    }
}
