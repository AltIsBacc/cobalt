use cobalt_common::utils::bi_channel::{self, CobaltReceiver, CobaltSender};
use cobalt_format::{common::Loader, world::World};
use tokio::sync::{broadcast, mpsc::{self, Receiver, Sender}, watch};

use crate::{
    commands::{CobaltCommand, CobaltResponse},
    error::CobaltServerResult,
    events::CobaltServerEvent,
    ticker::ServerTicker,
    traits::tickable::Tickable,
};

pub mod error;
pub mod world;
pub mod traits;
pub mod commands;
pub mod events;
pub mod ticker;

pub extern crate cobalt_common;

/// How the server sends packets back to one specific connected client.
/// Handed to the server in `CobaltCommand::ClientJoin`; the networking
/// layer is expected to construct one per connection and drive the other
/// end of it.
pub type ClientOutbound = mpsc::Sender<Vec<u8>>;

/// A cheap, `Clone`-able set of ways to talk to a running `CobaltServer`.
/// This is what embedders and callers hold on to -- never the server
/// itself, which lives inside its own task once spawned.
#[derive(Clone)]
pub struct CobaltServerHandle {
    pub ticks: mpsc::Sender<f32>,
    pub tick_rate: watch::Sender<u32>,

    pub commands: CobaltSender<CobaltCommand, CobaltResponse>,
    pub events: broadcast::Sender<CobaltServerEvent>,
}

impl CobaltServerHandle {
    /// Subscribe to server-emitted events (chat, join/leave, etc).
    /// Each subscriber gets every event sent from this point onward.
    pub fn subscribe_events(&self) -> broadcast::Receiver<CobaltServerEvent> {
        self.events.subscribe()
    }
}

struct CobaltServerChannels {
    commands: CobaltReceiver<CobaltCommand, CobaltResponse>,
    ticks: Receiver<f32>,
}

pub struct CobaltServer {
    world: World,
    elapsed_ticks: u64,

    channels: CobaltServerChannels,

    events_tx: broadcast::Sender<CobaltServerEvent>,
}

const EVENTS_CAPACITY: usize = 256;

impl CobaltServer {
    /// Build the server's state and a handle to talk to it, without
    /// running anything yet. Useful for embedding, tests, or driving the
    /// tick loop manually instead of via `run`/`spawn`.
    pub fn new<L: Loader<Output = World>>(
        loader: L,
        channel_capacity: usize,
        tick_capacity: usize,
        tick_speed: u32,
    ) -> CobaltServerResult<(Self, ServerTicker, CobaltServerHandle)> {
        let (host_io, server_io) =
            bi_channel::create_bi_channel::<CobaltCommand, CobaltResponse>(channel_capacity);

        let (tick_tx, tick_rx) = mpsc::channel::<f32>(tick_capacity);

        let ticker = ServerTicker::new(tick_speed);
        let tick_rate_tx = ticker.rate_handle();

        let (events_tx, _) = broadcast::channel(EVENTS_CAPACITY);

        let world = World::from_loader(loader)
            .map_err(|_| anyhow::anyhow!("failed to load world!"))?;

        let server = Self {
            world,
            elapsed_ticks: 0,
            channels: CobaltServerChannels {
                commands: server_io.1,
                ticks: tick_rx,
            },
            events_tx: events_tx.clone(),
        };

        let handle = CobaltServerHandle {
            ticks: tick_tx,
            tick_rate: tick_rate_tx,
            commands: host_io.0,
            events: events_tx,
        };

        Ok((server, ticker, handle))
    }

    /// Build and immediately spawn the server on a new tokio task,
    /// returning only a handle.
    pub fn spawn<L: Loader<Output = World> + Send + 'static>(
        loader: L,
        channel_capacity: usize,
        tick_capacity: usize,
        tick_speed: u32,
    ) -> CobaltServerResult<CobaltServerHandle> {
        let (server, ticker, handle) = Self::new(loader, channel_capacity, tick_capacity, tick_speed)?;
        tokio::spawn(server.run());
        ticker.spawn(handle.ticks.clone());
        Ok(handle)
    }

    pub async fn run(mut self) {
        loop {
            tokio::select! {
                biased;

                Ok(req) = self.channels.commands.recv() => {
                    let response = self.handle_command(&req.payload);
                    let _ = req.reply(response);
                }

                Some(dt) = self.channels.ticks.recv() => {
                    self.tick(dt);
                    self.elapsed_ticks += 1;
                }

                else => break,
            }
        }
    }

    /// Broadcast an event to any subscribers.
    fn emit(&self, event: CobaltServerEvent) {
        let _ = self.events_tx.send(event);
    }

    fn handle_command(&mut self, command: &CobaltCommand) -> CobaltResponse {
        match command {
            CobaltCommand::ClientJoin { id, outbound: _ } => {
                self.emit(CobaltServerEvent::ClientJoined { id: *id });
                CobaltResponse::Acknowledged
            }

            CobaltCommand::ClientLeave { id } => {
                self.emit(CobaltServerEvent::ClientLeft { id: *id });
                CobaltResponse::Acknowledged
            }

            CobaltCommand::ClientPacket { id: _, packet: _ } => {
                // TODO: interpret the packet once cobalt-protocol defines
                // a real packet type. For now, just acknowledge.
                CobaltResponse::Acknowledged
            }
        }
    }
}

impl Tickable for CobaltServer {
    fn tick(&mut self, dt: f32) {
        self.world.tick(dt);
    }
}
