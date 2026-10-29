use std::{net::SocketAddr, ops::ControlFlow, time::Duration};

use cobalt_format::World;
use tokio::{
    sync::{broadcast, mpsc},
    task::{JoinError, JoinHandle},
    time::{self, Instant, Interval, MissedTickBehavior},
};

use crate::{
    client::Clients,
    command::Command,
    config::{ServerConfig, validate_tick_rate},
    error::{ServerError, ServerResult},
    event::Event,
    handle::ServerHandle,
    log::debug_log,
    net::Listener,
    status::Status,
};

enum Wake {
    Tick,
    Command(Option<Command>),
}

pub struct Server {
    world: World,
    config: ServerConfig,
    clients: Clients,
    ticks: u64,
    started: Instant,
    commands: mpsc::Receiver<Command>,
    events: broadcast::Sender<Event>,
}

impl Server {
    pub fn new(world: World, config: ServerConfig) -> ServerResult<(Self, ServerHandle)> {
        config.validate()?;

        let (commands_tx, commands_rx) = mpsc::channel(config.command_capacity);
        let (events_tx, _) = broadcast::channel(config.event_capacity);

        let server = Self {
            world,
            config,
            clients: Clients::new(config.max_players),
            ticks: 0,
            started: Instant::now(),
            commands: commands_rx,
            events: events_tx.clone(),
        };

        Ok((server, ServerHandle::new(commands_tx, events_tx)))
    }

    pub fn spawn(world: World, config: ServerConfig) -> ServerResult<RunningServer> {
        let (server, handle) = Self::new(world, config)?;
        let listener = config
            .listen
            .map(|addr| Listener::bind(addr, handle.clone(), config.handshake_timeout))
            .transpose()?;
        let task = tokio::spawn(server.run());

        Ok(RunningServer { handle, task, listener })
    }

    pub async fn run(mut self) {
        self.started = Instant::now();

        let mut rate = self.config.tick_rate;
        let mut ticker = Self::ticker(rate);

        loop {
            let wake = tokio::select! {
                _ = ticker.tick() => Wake::Tick,
                command = self.commands.recv() => Wake::Command(command),
            };

            match wake {
                Wake::Tick => self.tick(),
                Wake::Command(None) => break,
                Wake::Command(Some(command)) => {
                    if self.dispatch(command).is_break() {
                        break;
                    }
                    if rate != self.config.tick_rate {
                        rate = self.config.tick_rate;
                        ticker = Self::ticker(rate);
                    }
                }
            }
        }

        self.emit(Event::Stopped);
    }

    fn ticker(rate: u32) -> Interval {
        let mut ticker = time::interval(Duration::from_secs(1) / rate);
        ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
        ticker
    }

    fn tick(&mut self) {
        self.ticks += 1;
    }

    fn dispatch(&mut self, command: Command) -> ControlFlow<()> {
        match command {
            Command::Join { reply } => {
                let result = self.clients.insert().ok_or(ServerError::Full);
                if let Ok(id) = &result {
                    self.emit(Event::ClientJoined { id: *id });
                }
                let _ = reply.send(result);
            }
            Command::Leave { id, reply } => {
                let result = if self.clients.remove(id) {
                    self.emit(Event::ClientLeft { id });
                    Ok(())
                } else {
                    Err(ServerError::UnknownClient(id))
                };
                let _ = reply.send(result);
            }
            Command::Status { reply } => {
                let _ = reply.send(self.status());
            }
            Command::SetTickRate { rate, reply } => {
                let result = validate_tick_rate(rate);
                if result.is_ok() {
                    self.config.tick_rate = rate;
                    debug_log!("tick rate set to {rate}");
                }
                let _ = reply.send(result);
            }
            Command::Shutdown { reply } => {
                let _ = reply.send(());
                return ControlFlow::Break(());
            }
        }

        ControlFlow::Continue(())
    }

    fn status(&self) -> Status {
        Status {
            world_name: self.world.header.name.clone(),
            players: self.clients.count(),
            max_players: self.clients.capacity(),
            tick_rate: self.config.tick_rate,
            ticks: self.ticks,
            uptime: self.started.elapsed(),
        }
    }

    fn emit(&self, event: Event) {
        debug_log!("event: {event:?}");
        let _ = self.events.send(event);
    }
}

pub struct RunningServer {
    handle: ServerHandle,
    task: JoinHandle<()>,
    listener: Option<Listener>,
}

impl RunningServer {
    pub const fn handle(&self) -> &ServerHandle {
        &self.handle
    }

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
        if let Some(listener) = listener {
            listener.stop().await;
        }
        let _ = handle.shutdown().await;
        task.await
    }
}

#[cfg(test)]
mod tests {
    use cobalt_format::world::header::{Difficulty, Size};

    use super::*;
    use crate::event::Events;

    fn world() -> World {
        World::new("test", Size::new(4, 4).unwrap(), Difficulty::Normal).unwrap()
    }

    fn config(max_players: u8) -> ServerConfig {
        ServerConfig { max_players, ..ServerConfig::default() }
    }

    async fn next(events: &mut Events) -> Event {
        events.recv().await.unwrap()
    }

    #[tokio::test(start_paused = true)]
    async fn clients_join_and_leave() {
        let running = Server::spawn(world(), config(2)).unwrap();
        let handle = running.handle().clone();

        let a = handle.join().await.unwrap();
        let b = handle.join().await.unwrap();
        assert_ne!(a, b);
        assert!(matches!(handle.join().await, Err(ServerError::Full)));

        handle.leave(a).await.unwrap();
        assert!(matches!(
            handle.leave(a).await,
            Err(ServerError::UnknownClient(id)) if id == a
        ));

        let status = handle.status().await.unwrap();
        assert_eq!((status.players, status.max_players), (1, 2));
        assert_eq!(status.world_name, "test");

        assert_eq!(handle.join().await.unwrap(), a);
        running.shutdown().await.unwrap();
    }

    #[tokio::test(start_paused = true)]
    async fn events_are_broadcast_in_order() {
        let running = Server::spawn(world(), config(2)).unwrap();
        let handle = running.handle().clone();
        let mut events = handle.subscribe();

        let id = handle.join().await.unwrap();
        handle.leave(id).await.unwrap();
        running.shutdown().await.unwrap();

        assert_eq!(next(&mut events).await, Event::ClientJoined { id });
        assert_eq!(next(&mut events).await, Event::ClientLeft { id });
        assert_eq!(next(&mut events).await, Event::Stopped);
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

        assert!(matches!(handle.join().await, Err(ServerError::Stopped)));
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
            ServerConfig { command_capacity: 0, ..ServerConfig::default() },
            ServerConfig { event_capacity: 0, ..ServerConfig::default() },
            ServerConfig { handshake_timeout: Duration::ZERO, ..ServerConfig::default() },
        ] {
            assert!(matches!(Server::new(world(), config), Err(ServerError::InvalidConfig(_))));
        }
    }
}
