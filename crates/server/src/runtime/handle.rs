use tokio::sync::{broadcast, mpsc, oneshot};

use crate::error::{ServerError, ServerResult};
use crate::model::{Event, Events, Status};

pub(crate) enum Control {
    Status(oneshot::Sender<ServerResult<Status>>),
    SetTickRate { rate: u32, reply: oneshot::Sender<ServerResult> },
    Shutdown(oneshot::Sender<ServerResult>),
}

#[derive(Debug, Clone)]
pub struct ServerHandle {
    control: mpsc::Sender<Control>,
    events: broadcast::Sender<Event>,
}

impl ServerHandle {
    pub(crate) const fn new(
        control: mpsc::Sender<Control>,
        events: broadcast::Sender<Event>,
    ) -> Self {
        Self { control, events }
    }

    #[must_use]
    pub fn subscribe(&self) -> Events {
        self.events.subscribe()
    }

    pub async fn status(&self) -> ServerResult<Status> {
        self.request(Control::Status).await
    }

    pub async fn set_tick_rate(&self, rate: u32) -> ServerResult {
        self.request(|reply| Control::SetTickRate { rate, reply }).await
    }

    pub async fn shutdown(&self) -> ServerResult {
        self.request(Control::Shutdown).await
    }

    async fn request<T>(
        &self,
        build: impl FnOnce(oneshot::Sender<ServerResult<T>>) -> Control,
    ) -> ServerResult<T> {
        let (reply, response) = oneshot::channel();
        if self.control.send(build(reply)).await.is_err() {
            return Err(ServerError::Stopped);
        }
        response.await.unwrap_or(Err(ServerError::Stopped))
    }
}
