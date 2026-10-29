use tokio::sync::oneshot;

use crate::{client::ClientId, error::ServerResult, status::Status};

pub(crate) enum Command {
    Join { reply: oneshot::Sender<ServerResult<ClientId>> },
    Leave { id: ClientId, reply: oneshot::Sender<ServerResult> },
    Status { reply: oneshot::Sender<Status> },
    SetTickRate { rate: u32, reply: oneshot::Sender<ServerResult> },
    Shutdown { reply: oneshot::Sender<()> },
}
