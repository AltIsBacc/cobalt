use std::time::Duration;

use cobalt_format::World;

use super::clock::Clock;
use super::items::Items;
use super::section::Streamed;
use super::slots::Slots;
use super::stage::Stage;
use crate::config::ServerConfig;
use crate::model::Status;

pub(super) struct State {
    pub(super) world: World,
    pub(super) clock: Clock,
    pub(super) slots: Slots<Stage>,
    pub(super) streams: Box<[Streamed]>,
    pub(super) items: Items,
    pub(super) tick_rate: u32,
    pub(super) ticks: u64,
}

impl State {
    pub(super) fn new(world: World, config: &ServerConfig) -> Self {
        Self {
            clock: Clock::new(&world),
            streams: (0..config.max_players).map(|_| Streamed::new(world.header.size)).collect(),
            world,
            slots: Slots::new(config.max_players),
            items: Items::new(),
            tick_rate: config.tick_rate,
            ticks: 0,
        }
    }

    pub(super) fn status(&self, uptime: Duration) -> Status {
        Status {
            world_name: self.world.header.name.clone(),
            players: self.slots.len(),
            max_players: self.slots.capacity(),
            tick_rate: self.tick_rate,
            ticks: self.ticks,
            uptime,
        }
    }
}
