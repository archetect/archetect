use std::fs;

use camino::Utf8PathBuf;

use archetect_api::{ClientIoHandle, ClientMessage, WriteDirectoryInfo};

pub fn handle_write_directory<CIO: ClientIoHandle>(prompt_info: WriteDirectoryInfo, client_handle: CIO) {
    let destination = Utf8PathBuf::from(prompt_info.path);
    if !destination.exists() {
        match fs::create_dir_all(destination) {
            Ok(_success) => {}
            Err(error) => {
                client_handle.send(ClientMessage::Error(error.to_string()));
                return;
            }
        }
    }
    client_handle.send(ClientMessage::Ack);
}
