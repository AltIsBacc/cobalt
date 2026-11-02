use tokio::sync::{broadcast, mpsc, oneshot};

use crate::client::ClientId;
use crate::command::Command;
use crate::error::{ServerError, ServerResult};
use crate::event::{Event, Events};
use crate::player::Player;
use crate::status::Status;

#[derive(Debug, Clone)]
pub struct ServerHandle {
    commands: mpsc::Sender<Command>,
    events: broadcast::Sender<Event>,
}

impl ServerHandle {
    pub(crate) fn new(
        commands: mpsc::Sender<Command>,
        events: broadcast::Sender<Event>,
    ) -> Self {
        Self { commands, events }
    }

    #[must_use]
    pub fn subscribe(&self) -> Events {
        self.events.subscribe()
    }

    pub async fn join(&self) -> ServerResult<ClientId> {
        self.request(|reply| Command::Join { reply }).await?
    }

    pub async fn login(&self, id: ClientId, player: Player) -> ServerResult {
        self.request(|reply| Command::Login { id, player, reply }).await?
    }

    pub async fn leave(&self, id: ClientId) -> ServerResult {
        self.request(|reply| Command::Leave { id, reply }).await?
    }

    pub async fn status(&self) -> ServerResult<Status> {
        self.request(|reply| Command::Status { reply }).await
    }

    pub async fn set_tick_rate(&self, rate: u32) -> ServerResult {
        self.request(|reply| Command::SetTickRate { rate, reply }).await?
    }

    pub async fn shutdown(&self) -> ServerResult {
        self.request(|reply| Command::Shutdown { reply }).await
    }

    async fn request<T>(
        &self,
        build: impl FnOnce(oneshot::Sender<T>) -> Command,
    ) -> ServerResult<T> {
        let (reply, response) = oneshot::channel();
        self.commands.send(build(reply)).await.map_err(|_| ServerError::Stopped)?;
        response.await.map_err(|_| ServerError::Stopped)
    }
}
