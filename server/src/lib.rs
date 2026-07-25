use cobalt_common::utils::bi_channel::{self, CobaltReceiver, CobaltSender};
use cobalt_format::{common::Loader, world::World};
use getset::Getters;
use tokio::sync::{mpsc::{self, Receiver, Sender}, watch};

use crate::{error::CobaltResult, events::CobaltServerEvent, messages::{CobaltCommand, CobaltResponse}, ticker::ServerTicker, traits::tickable::Tickable};

pub mod error;
pub mod world;
pub mod traits;
pub mod messages;
pub mod events;
pub mod ticker;

pub extern crate cobalt_common;

#[derive(Clone)]
pub struct CobaltServerTxChannels {
    pub commands: CobaltSender<CobaltCommand, CobaltResponse>,
    pub ticks: Sender<f32>,
    pub tick_rate: watch::Sender<u32>,
}

pub struct CobaltServerRxChannels {
    pub commands: CobaltReceiver<CobaltCommand, CobaltResponse>,
    pub ticks: Receiver<f32>,
}

#[derive(Getters)]
#[getset(get = "pub")]
pub struct CobaltServer {
    world: World,
    elapsed_ticks: u64,

    ticker: ServerTicker,

    channels_tx: CobaltServerTxChannels,
    channels_rx: CobaltServerRxChannels,
}

impl CobaltServer {
    pub fn new<L: Loader<Output = World>>(
        loader: L,
        channel_capacity: usize,
        tick_capacity: usize,
        tick_speed: u32,
    ) -> CobaltResult<Self> {
        let (
            host_io,
            server_io
        ) = bi_channel::create_bi_channel::<
            CobaltCommand, CobaltResponse
        >(channel_capacity);

        let (
            tick_tx,
            tick_rx
        ) = mpsc::channel::<f32>(tick_capacity);

        let ticker = ServerTicker::new(tick_speed);
        let tick_rate_handle = ticker.rate_handle();

        let server = Self {
            world: World::from_loader(loader)
                .map_err(|_| anyhow::anyhow!("failed to load world!"))?,
            elapsed_ticks: 0,
            ticker,
            channels_tx: CobaltServerTxChannels {
                commands: host_io.0,
                ticks: tick_tx,
                tick_rate: tick_rate_handle,
            },
            channels_rx: CobaltServerRxChannels {
                commands: server_io.1,
                ticks: tick_rx,
            },
        };

        Ok(server)
    }

    pub fn server_handle(&self) -> CobaltServerTxChannels {
        self.channels_tx.clone()
    }

    pub async fn run_loop(mut self) {
        let _ = self.ticker.spawn(self.channels_tx.ticks.clone());

        loop {
            tokio::select! {
                biased;

                Ok(req) = self.channels_rx.commands.recv() => {
                    match req.payload {
                        CobaltCommand::Event(ref e) => {
                            if self.handle_event(e) {
                                let _ = req.reply(CobaltResponse::Acknowledged);
                            }
                        },
                    }
                }

                Some(dt) = self.channels_rx.ticks.recv() => {
                    self.tick(dt);
                    self.elapsed_ticks += 1;
                }

                else => break,
            }
        }
    }

    fn handle_event(&mut self, event: &CobaltServerEvent) -> bool {
        match event {
            _ => false
        }
    }
}

impl Tickable for CobaltServer {
    fn tick(&mut self, dt: f32) {
        self.world.tick(dt);
    }
}
