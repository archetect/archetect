use std::fmt::Debug;
use std::sync::{Arc, mpsc, Mutex};
use std::sync::mpsc::{Receiver, SyncSender};

use crate::{CommandRequest, CommandResponse};

pub trait IoDriver: Debug + Send + Sync + 'static {
    fn send(&self, request: CommandRequest);

    fn responses(&self) -> Arc<Mutex<Receiver<CommandResponse>>>;

    fn receive(&self) -> CommandResponse {
        self.responses().lock().expect("Lock Error")
            .recv().expect("Receive Error")
    }
}

impl<T: IoDriver> From<T> for Box<dyn IoDriver> {
    fn from(value: T) -> Self {
        Box::new(value)
    }
}

pub fn api_driver_and_handle() -> (ApiIoDriver, ApiIoHandle) {
    let (requests_tx, requests_rx) = mpsc::sync_channel(1);
    let (responses_tx, responses_rx) = mpsc::sync_channel(1);
    let driver = ApiIoDriver {
        requests_tx,
        responses_rx: Arc::new(Mutex::new(responses_rx)),
    };
    let handle = ApiIoHandle {
        responses_tx,
        requests_rx,
    };
    (driver, handle)
}

#[derive(Debug)]
pub struct ApiIoDriver {
    requests_tx: SyncSender<CommandRequest>,
    responses_rx: Arc<Mutex<Receiver<CommandResponse>>>,
}

impl IoDriver for ApiIoDriver {
    fn send(&self, request: CommandRequest) {
        self.requests_tx.send(request).expect("Send Error");
    }

    fn responses(&self) -> Arc<Mutex<Receiver<CommandResponse>>> {
        self.responses_rx.clone()
    }
}

pub struct ApiIoHandle {
    requests_rx: Receiver<CommandRequest>,
    responses_tx: SyncSender<CommandResponse>,
}

impl ApiIoHandle {
    pub fn respond(&self, response: CommandResponse) {
        self.responses_tx.send(response).expect("Send Error")
    }

    pub fn requests(&self) -> &Receiver<CommandRequest> {
        &self.requests_rx
    }

    pub fn receive(&self) -> CommandRequest {
        self.requests_rx.recv().expect("Receive Error")
    }
}