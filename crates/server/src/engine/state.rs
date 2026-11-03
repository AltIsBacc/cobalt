use std::time::Duration;

use cobalt_format::World;

use super::clock::Clock;
use super::slots::Slots;
use super::stage::Stage;
use crate::config::ServerConfig;
use crate::model::Status;

pub(super) struct State {
    pub(super) world: World,
    pub(super) clock: Clock,
    pub(super) slots: Slots<Stage>,
    pub(super) tick_rate: u32,
    pub(super) ticks: u64,
}

impl State {
    pub(super) fn new(world: World, config: &ServerConfig) -> Self {
        Self {
            clock: Clock::new(&world),
            world,
            slots: Slots::new(config.max_players),
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
