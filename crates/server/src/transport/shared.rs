use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use cobalt_format::World;
use cobalt_protocol::{Frame, ProtocolResult};
use tokio::sync::{Notify, broadcast, mpsc, watch};
use tokio::time::Instant;

use crate::client::ClientId;
use crate::config::ServerConfig;
use crate::engine::{Admission, ClientError, Engine, Flow, Outbox};
use crate::error::{ServerError, ServerResult};
use crate::event::{Event, Events};
use crate::log::debug_log;
use crate::status::Status;

pub(crate) type Queue = mpsc::Sender<Arc<[u8]>>;

struct State {
    engine: Engine,
    outbox: Outbox,
    queues: Box<[Option<Queue>]>,
}

pub(crate) struct Shared {
    state: Mutex<State>,
    events: broadcast::Sender<Event>,
    shutdown: watch::Sender<bool>,
    released: Notify,
    started: Instant,
    handshake_timeout: Duration,
    outbound_capacity: usize,
}

impl Shared {
    pub(crate) fn new(world: World, config: &ServerConfig) -> Self {
        let state = State {
            engine: Engine::new(world, config),
            outbox: Outbox::default(),
            queues: (0..config.max_players).map(|_| None).collect(),
        };

        Self {
            state: Mutex::new(state),
            events: broadcast::channel(config.event_capacity).0,
            shutdown: watch::channel(false).0,
            released: Notify::new(),
            started: Instant::now(),
            handshake_timeout: config.handshake_timeout,
            outbound_capacity: config.outbound_capacity,
        }
    }

    pub(crate) const fn handshake_timeout(&self) -> Duration {
        self.handshake_timeout
    }

    pub(crate) const fn outbound_capacity(&self) -> usize {
        self.outbound_capacity
    }

    pub(crate) fn subscribe(&self) -> Events {
        self.events.subscribe()
    }

    pub(crate) fn shutdown_signal(&self) -> watch::Receiver<bool> {
        self.shutdown.subscribe()
    }

    pub(crate) fn admit(
        &self,
        frame: Frame<'_>,
        queue: &mut Option<Queue>,
    ) -> ProtocolResult<Admission> {
        self.with(|engine, outbox, queues| {
            let admission = engine.admit(frame, outbox)?;
            if let Admission::Admitted(id) = admission
                && let Some(slot) = queues.get_mut(usize::from(id.get()))
            {
                *slot = queue.take();
            }
            Ok(admission)
        })
    }

    pub(crate) fn receive(&self, id: ClientId, frame: Frame<'_>) -> Result<Flow, ClientError> {
        self.with(|engine, outbox, _| engine.receive(id, frame, outbox))
    }

    pub(crate) fn release(&self, id: ClientId) {
        self.with(|engine, outbox, queues| {
            if let Some(slot) = queues.get_mut(usize::from(id.get())) {
                *slot = None;
            }
            engine.release(id, outbox);
        });
        self.released.notify_waiters();
    }

    pub(crate) fn tick(&self) -> u32 {
        self.state().engine.tick()
    }

    pub(crate) fn status(&self) -> ServerResult<Status> {
        let state = self.state();
        if !state.engine.is_open() {
            return Err(ServerError::Stopped);
        }
        Ok(state.engine.status(self.started.elapsed()))
    }

    pub(crate) fn set_tick_rate(&self, rate: u32) -> ServerResult {
        let mut state = self.state();
        if !state.engine.is_open() {
            return Err(ServerError::Stopped);
        }
        state.engine.set_tick_rate(rate)
    }

    pub(crate) fn shutdown(&self) -> ServerResult {
        if !self.state().engine.close() {
            return Err(ServerError::Stopped);
        }
        self.shutdown.send_replace(true);
        Ok(())
    }

    pub(crate) async fn drained(&self) {
        loop {
            let released = self.released.notified();
            if self.state().engine.is_empty() {
                return;
            }
            released.await;
        }
    }

    pub(crate) fn publish_stopped(&self) {
        debug_log!("event: {:?}", Event::Stopped);
        let _ = self.events.send(Event::Stopped);
    }

    fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn with<R>(&self, f: impl FnOnce(&mut Engine, &mut Outbox, &mut [Option<Queue>]) -> R) -> R {
        let mut state = self.state();
        let State { engine, outbox, queues } = &mut *state;
        let result = f(engine, outbox, queues);

        outbox.drain(
            |to, bytes| {
                let Some(slot) = queues.get_mut(usize::from(to.get())) else { return };
                if slot.as_ref().is_some_and(|queue| queue.try_send(Arc::from(bytes)).is_err()) {
                    debug_log!("{to} dropped: outbound queue is full or closed");
                    *slot = None;
                }
            },
            |event| {
                debug_log!("event: {event:?}");
                let _ = self.events.send(event);
            },
        );

        result
    }
}

pub(crate) async fn stopped(signal: &mut watch::Receiver<bool>) {
    let _ = signal.wait_for(|&stopped| stopped).await;
}
