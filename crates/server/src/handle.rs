use std::fmt;
use std::sync::Arc;

use crate::error::ServerResult;
use crate::event::Events;
use crate::status::Status;
use crate::transport::Shared;

struct Guard(Arc<Shared>);

impl Drop for Guard {
    fn drop(&mut self) {
        let _ = self.0.shutdown();
    }
}

#[derive(Clone)]
pub struct ServerHandle {
    guard: Arc<Guard>,
}

impl ServerHandle {
    pub(crate) fn new(shared: Arc<Shared>) -> Self {
        Self { guard: Arc::new(Guard(shared)) }
    }

    #[must_use]
    pub fn subscribe(&self) -> Events {
        self.guard.0.subscribe()
    }

    pub async fn status(&self) -> ServerResult<Status> {
        self.guard.0.status()
    }

    pub async fn set_tick_rate(&self, rate: u32) -> ServerResult {
        self.guard.0.set_tick_rate(rate)
    }

    pub async fn shutdown(&self) -> ServerResult {
        self.guard.0.shutdown()
    }
}

impl fmt::Debug for ServerHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ServerHandle").finish_non_exhaustive()
    }
}
