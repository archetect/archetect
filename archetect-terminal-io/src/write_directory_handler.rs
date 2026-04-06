use std::fs;


use camino::Utf8PathBuf;
use log::debug;

use archetect_api::{ClientMessage, WriteDirectoryInfo};
use crate::responder::Responder;

pub fn handle_write_directory(write_info: WriteDirectoryInfo, responses: &dyn Responder) {
    let path = Utf8PathBuf::from(&write_info.path);

    if !path.exists() {
        debug!("Creating directory {:?}", path);
        match fs::create_dir_all(&path) {
            Ok(()) => {}
            Err(error) => {
                responses.respond(ClientMessage::Error(error.to_string()));
                return;
            }
        }
    }

    responses.respond(ClientMessage::Ack);
}
