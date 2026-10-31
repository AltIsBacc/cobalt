use tokio::sync::broadcast;

use crate::client::ClientId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    ClientJoined { id: ClientId },
    ClientLoggedIn { id: ClientId },
    ClientLeft { id: ClientId },
    Stopped,
}

pub type Events = broadcast::Receiver<Event>;
