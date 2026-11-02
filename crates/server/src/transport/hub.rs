use std::time::Duration;

use tokio::sync::{mpsc, oneshot, watch};

use crate::engine::Admission;
use crate::model::ClientId;

pub(crate) type Queue = mpsc::Sender<Vec<u8>>;
pub(crate) type Inbox = mpsc::Receiver<Vec<u8>>;

pub(crate) enum Traffic {
    Join { queue: Queue, reply: oneshot::Sender<Admission> },
    Input { id: ClientId, frames: Vec<u8> },
    Leave { id: ClientId },
}

#[derive(Clone)]
pub(crate) struct Hub {
    pub(crate) traffic: mpsc::Sender<Traffic>,
    pub(crate) shutdown: watch::Receiver<bool>,
    pub(crate) handshake_timeout: Duration,
    pub(crate) outbound_capacity: usize,
}

pub(crate) async fn stopped(signal: &mut watch::Receiver<bool>) {
    let _ = signal.wait_for(|&stopped| stopped).await;
}
