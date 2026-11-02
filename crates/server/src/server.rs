use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use cobalt_format::World;
use tokio::task::{JoinError, JoinHandle};
use tokio::time::{self, Interval, MissedTickBehavior};

use crate::config::ServerConfig;
use crate::error::ServerResult;
use crate::handle::ServerHandle;
use crate::transport::{Listener, Shared, stopped};

pub struct Server {
    shared: Arc<Shared>,
    tick_rate: u32,
}

impl Server {
    pub fn try_new(world: World, config: ServerConfig) -> ServerResult<(Self, ServerHandle)> {
        config.validate()?;

        let shared = Arc::new(Shared::new(world, &config));
        let handle = ServerHandle::new(Arc::clone(&shared));

        Ok((Self { shared, tick_rate: config.tick_rate }, handle))
    }

    pub fn spawn(world: World, config: ServerConfig) -> ServerResult<RunningServer> {
        let (server, handle) = Self::try_new(world, config)?;
        let listener = config
            .listen
            .map(|addr| Listener::bind(addr, Arc::clone(&server.shared)))
            .transpose()?;
        let task = tokio::spawn(server.run());

        Ok(RunningServer { handle, task, listener })
    }

    pub async fn run(self) {
        let mut shutdown = self.shared.shutdown_signal();
        let mut rate = self.tick_rate;
        let mut ticker = interval(rate);

        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    let current = self.shared.tick();
                    if current != rate {
                        rate = current;
                        ticker = interval(rate);
                    }
                }
                () = stopped(&mut shutdown) => break,
            }
        }

        self.shared.drained().await;
        self.shared.publish_stopped();
    }
}

fn interval(rate: u32) -> Interval {
    let mut ticker = time::interval(Duration::from_secs(1) / rate);
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
    ticker
}

pub struct RunningServer {
    handle: ServerHandle,
    task: JoinHandle<()>,
    listener: Option<Listener>,
}

impl RunningServer {
    #[must_use]
    pub const fn handle(&self) -> &ServerHandle {
        &self.handle
    }

    #[must_use]
    pub fn local_addr(&self) -> Option<SocketAddr> {
        self.listener.as_ref().map(Listener::local_addr)
    }

    pub async fn join(self) -> Result<(), JoinError> {
        let Self { handle, task, listener } = self;
        drop(handle);
        let result = task.await;
        if let Some(listener) = listener {
            listener.stop().await;
        }
        result
    }

    pub async fn shutdown(self) -> Result<(), JoinError> {
        let Self { handle, task, listener } = self;
        let _ = handle.shutdown().await;
        if let Some(listener) = listener {
            listener.stop().await;
        }
        task.await
    }
}

#[cfg(test)]
mod tests {
    use cobalt_format::world::header::{Difficulty, Size};

    use super::*;
    use crate::error::ServerError;

    fn world() -> World {
        World::try_new("test", Size::try_new(4, 4).unwrap(), Difficulty::Normal).unwrap()
    }

    fn config(max_players: u8) -> ServerConfig {
        ServerConfig { max_players, ..ServerConfig::default() }
    }

    #[tokio::test(start_paused = true)]
    async fn ticks_follow_the_configured_rate() {
        let config = ServerConfig { tick_rate: 20, ..ServerConfig::default() };
        let running = Server::spawn(world(), config).unwrap();

        time::sleep(Duration::from_secs(1)).await;
        let status = running.handle().status().await.unwrap();

        assert!((19..=22).contains(&status.ticks), "ticks: {}", status.ticks);
        assert!(status.uptime >= Duration::from_secs(1));
        running.shutdown().await.unwrap();
    }

    #[tokio::test(start_paused = true)]
    async fn tick_rate_can_change_while_running() {
        let running = Server::spawn(world(), config(1)).unwrap();
        let handle = running.handle().clone();

        assert!(matches!(handle.set_tick_rate(0).await, Err(ServerError::InvalidConfig(_))));
        assert!(matches!(
            handle.set_tick_rate(ServerConfig::MAX_TICK_RATE + 1).await,
            Err(ServerError::InvalidConfig(_))
        ));
        assert_eq!(handle.status().await.unwrap().tick_rate, 60);

        handle.set_tick_rate(10).await.unwrap();
        assert_eq!(handle.status().await.unwrap().tick_rate, 10);
        running.shutdown().await.unwrap();
    }

    #[tokio::test(start_paused = true)]
    async fn handles_fail_after_shutdown() {
        let running = Server::spawn(world(), config(1)).unwrap();
        let handle = running.handle().clone();
        running.shutdown().await.unwrap();

        assert!(matches!(handle.set_tick_rate(30).await, Err(ServerError::Stopped)));
        assert!(matches!(handle.status().await, Err(ServerError::Stopped)));
    }

    #[tokio::test(start_paused = true)]
    async fn stops_when_every_handle_is_dropped() {
        let running = Server::spawn(world(), config(1)).unwrap();
        running.join().await.unwrap();
    }

    #[test]
    fn rejects_invalid_configs() {
        for config in [
            ServerConfig { tick_rate: 0, ..ServerConfig::default() },
            ServerConfig { max_players: 0, ..ServerConfig::default() },
            ServerConfig { outbound_capacity: 0, ..ServerConfig::default() },
            ServerConfig { event_capacity: 0, ..ServerConfig::default() },
            ServerConfig { handshake_timeout: Duration::ZERO, ..ServerConfig::default() },
        ] {
            assert!(matches!(Server::try_new(world(), config), Err(ServerError::InvalidConfig(_))));
        }
    }
}
