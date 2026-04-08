use std::fs;


use camino::Utf8PathBuf;
use log::debug;

use archetect_api::{ClientMessage, ExistingFilePolicy, WriteFileInfo};
use crate::responder::Responder;
use inquire::Confirm;

pub fn handle_write_file(write_info: WriteFileInfo, responses: &dyn Responder) {
    let path = Utf8PathBuf::from(&write_info.destination);

    if path.exists() {
        match write_info.existing_file_policy {
            ExistingFilePolicy::Overwrite => {
                debug!("Overwriting {:?}", path);
            }
            ExistingFilePolicy::Preserve => {
                debug!("Preserving {:?}", path);
                responses.respond(ClientMessage::Ack);
                return;
            }
            ExistingFilePolicy::Prompt => {
                let overwrite = Confirm::new(format!("Overwrite '{}'?", path).as_str())
                    .prompt_skippable()
                    .unwrap_or_default()
                    .unwrap_or_default();
                if !overwrite {
                    debug!("Preserving {:?}", path);
                    responses.respond(ClientMessage::Ack);
                    return;
                }
                debug!("Overwriting {:?}", path);
            }
        }
    } else {
        debug!("Writing {:?}", path);
    }

    match fs::write(&path, write_info.contents) {
        Ok(()) => {
            responses.respond(ClientMessage::Ack);
        }
        Err(error) => {
            responses.respond(ClientMessage::Error(error.to_string()));
        }
    }
}
