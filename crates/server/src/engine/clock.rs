use std::mem;

use cobalt_format::World;
use cobalt_format::world::header::flags::WorldFlag;
use cobalt_protocol::registry::Time;

use super::Outbox;
use super::schedule::System;
use super::state::State;

const DAY_LENGTH: f64 = 54_000.0;
const NIGHT_LENGTH: f64 = 32_400.0;
const TICKS_PER_SECOND: f64 = 60.0;

pub(super) struct Clock {
    day: bool,
    time: f64,
    unsynced_ticks: u32,
    due: bool,
}

impl Clock {
    pub(super) fn new(world: &World) -> Self {
        let header = &world.header;
        Self {
            day: header.flags.contains(&WorldFlag::DayTime),
            time: header.clock.time,
            unsynced_ticks: 0,
            due: false,
        }
    }

    pub(super) fn advance(&mut self, tick_rate: u32) {
        self.time += TICKS_PER_SECOND / f64::from(tick_rate);
        self.unsynced_ticks += 1;

        let length = if self.day { DAY_LENGTH } else { NIGHT_LENGTH };
        let flipped = self.time >= length;
        if flipped {
            self.time -= length;
            self.day = !self.day;
        }

        if flipped || self.unsynced_ticks >= tick_rate {
            self.unsynced_ticks = 0;
            self.due = true;
        }
    }

    pub(super) fn take_due(&mut self) -> bool {
        mem::take(&mut self.due)
    }

    pub(super) fn packet(&self) -> Time {
        Time { day_time: self.day, time: self.time as i32, sun_mod_y: 0, moon_mod_y: 0 }
    }
}

pub(super) struct ClockSystem;

impl System for ClockSystem {
    fn update(&self, state: &mut State) {
        state.clock.advance(state.tick_rate);
    }

    fn sync(&self, state: &mut State, out: &mut Outbox) {
        if state.clock.take_due() {
            out.broadcast(state.audience(None), state.clock.packet().into());
        }
    }
}
