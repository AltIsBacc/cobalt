use tokio::sync::oneshot;

use crate::client::ClientId;
use crate::error::ServerResult;
use crate::player::Player;
use crate::status::Status;

pub(crate) enum Command {
    Join { reply: oneshot::Sender<ServerResult<ClientId>> },
    Login { id: ClientId, player: Player, reply: oneshot::Sender<ServerResult> },
    Leave { id: ClientId, reply: oneshot::Sender<ServerResult> },
    Status { reply: oneshot::Sender<Status> },
    SetTickRate { rate: u32, reply: oneshot::Sender<ServerResult> },
    Shutdown { reply: oneshot::Sender<()> },
}
