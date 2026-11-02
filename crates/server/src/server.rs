use std::net::SocketAddr;
use std::time::Duration;

use cobalt_format::World;
use tokio::sync::{broadcast, mpsc, watch};
use tokio::task::{JoinError, JoinHandle};
use tokio::time::{self, Instant, Interval, MissedTickBehavior};

use crate::config::ServerConfig;
use crate::engine::{Admission, Engine, Flow, Outbox};
use crate::error::{ServerError, ServerResult};
use crate::event::Event;
use crate::handle::{Control, ServerHandle};
use crate::log::debug_log;
use crate::status::Status;
use crate::transport::{Hub, Listener, Queue, Traffic};

const CONTROL_CAPACITY: usize = 8;
const TRAFFIC_CAPACITY: usize = 1024;
const INTERNAL_ERROR: &str = "internal server error";

pub struct Server {
    engine: Engine,
    outbox: Outbox,
    queues: Box<[Option<Queue>]>,
    control: mpsc::Receiver<Control>,
    traffic: mpsc::Receiver<Traffic>,
    events: broadcast::Sender<Event>,
    shutdown: watch::Sender<bool>,
    hub: Hub,
    started: Instant,
}

impl Server {
    pub fn try_new(world: World, config: ServerConfig) -> ServerResult<(Self, ServerHandle)> {
        config.validate()?;

        let (control_tx, control) = mpsc::channel(CONTROL_CAPACITY);
        let (traffic_tx, traffic) = mpsc::channel(TRAFFIC_CAPACITY);
        let (events, _) = broadcast::channel(config.event_capacity);
        let (shutdown, shutdown_signal) = watch::channel(false);

        let server = Self {
            engine: Engine::new(world, &config),
            outbox: Outbox::new(config.max_players),
            queues: (0..config.max_players).map(|_| None).collect(),
            control,
            traffic,
            events: events.clone(),
            shutdown,
            hub: Hub {
                traffic: traffic_tx,
                shutdown: shutdown_signal,
                handshake_timeout: config.handshake_timeout,
                outbound_capacity: config.outbound_capacity,
            },
            started: Instant::now(),
        };

        Ok((server, ServerHandle::new(control_tx, events)))
    }

    pub fn spawn(world: World, config: ServerConfig) -> ServerResult<RunningServer> {
        let (server, handle) = Self::try_new(world, config)?;
        let listener = config
            .listen
            .map(|addr| Listener::bind(addr, server.hub.clone()))
            .transpose()?;
        let task = tokio::spawn(server.run());

        Ok(RunningServer { handle, task, listener })
    }

    pub async fn run(mut self) {
        let mut rate = self.engine.tick_rate();
        let mut ticker = interval(rate);
        let mut controlled = true;

        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    let current = self.engine.tick(&mut self.outbox);
                    self.flush();
                    if current != rate {
                        rate = current;
                        ticker = interval(rate);
                    }
                }
                control = self.control.recv(), if controlled => match control {
                    Some(control) => self.on_control(control),
                    None => {
                        controlled = false;
                        self.close();
                    }
                },
                Some(traffic) = self.traffic.recv() => self.on_traffic(traffic),
            }

            if !self.engine.is_open() && self.engine.is_empty() {
                break;
            }
        }

        debug_log!("event: {:?}", Event::Stopped);
        let _ = self.events.send(Event::Stopped);
    }

    fn on_control(&mut self, control: Control) {
        match control {
            Control::Status(reply) => {
                let _ = reply.send(self.status());
            }
            Control::SetTickRate { rate, reply } => {
                let _ = reply.send(self.set_tick_rate(rate));
            }
            Control::Shutdown(reply) => {
                let closed = self.close();
                let _ = reply.send(if closed { Ok(()) } else { Err(ServerError::Stopped) });
            }
        }
    }

    fn on_traffic(&mut self, traffic: Traffic) {
        let closing = match traffic {
            Traffic::Join { queue, reply } => {
                let _ = reply.send(self.join(queue));
                None
            }
            Traffic::Input { id, frames } => {
                let flow = match self.engine.receive(id, &frames, &mut self.outbox) {
                    Ok(flow) => flow,
                    Err(error) => {
                        debug_log!("{id} dropped: {error}");
                        Flow::Close
                    }
                };
                (flow == Flow::Close).then_some(id)
            }
            Traffic::Leave { id } => {
                self.queues[id.index()] = None;
                self.engine.release(id, &mut self.outbox);
                None
            }
        };

        self.flush();
        if let Some(id) = closing {
            self.queues[id.index()] = None;
        }
    }

    fn join(&mut self, queue: Queue) -> Admission {
        match self.engine.join(&mut self.outbox) {
            Ok(Admission::Admitted(id)) => {
                self.queues[id.index()] = Some(queue);
                Admission::Admitted(id)
            }
            Ok(refused) => refused,
            Err(error) => {
                debug_log!("join failed: {error}");
                Admission::Refused(INTERNAL_ERROR)
            }
        }
    }

    fn flush(&mut self) {
        let Self { outbox, queues, events, .. } = self;

        outbox.drain(
            |to, bytes| {
                let slot = &mut queues[to.index()];
                if slot.as_ref().is_some_and(|queue| queue.try_send(bytes).is_err()) {
                    debug_log!("{to} dropped: outbound queue is full or closed");
                    *slot = None;
                }
            },
            |event| {
                debug_log!("event: {event:?}");
                let _ = events.send(event);
            },
        );
    }

    fn close(&mut self) -> bool {
        let was_open = self.engine.close();
        if was_open {
            self.queues.fill_with(|| None);
            self.shutdown.send_replace(true);
        }
        was_open
    }

    fn status(&self) -> ServerResult<Status> {
        if !self.engine.is_open() {
            return Err(ServerError::Stopped);
        }
        Ok(self.engine.status(self.started.elapsed()))
    }

    fn set_tick_rate(&mut self, rate: u32) -> ServerResult {
        if !self.engine.is_open() {
            return Err(ServerError::Stopped);
        }
        self.engine.set_tick_rate(rate)
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
