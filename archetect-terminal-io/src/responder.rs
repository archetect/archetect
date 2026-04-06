use std::sync::mpsc::SyncSender;

use archetect_api::ClientMessage;

pub trait Responder {
    fn respond(&self, message: ClientMessage);
}

impl Responder for SyncSender<ClientMessage> {
    fn respond(&self, message: ClientMessage) {
        self.send(message).expect("Channel Send Error");
    }
}
