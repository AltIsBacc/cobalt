use cobalt_format::World;
use cobalt_format::world::header::flags::WorldFlag;
use cobalt_protocol::registry::Time;

const DAY_LENGTH: f64 = 54_000.0;
const NIGHT_LENGTH: f64 = 32_400.0;
const TICKS_PER_SECOND: f64 = 60.0;

pub(super) struct Clock {
    day: bool,
    time: f64,
}

impl Clock {
    pub(super) fn new(world: &World) -> Self {
        let header = &world.header;
        Self { day: header.flags.contains(&WorldFlag::DayTime), time: header.clock.time }
    }

    pub(super) fn advance(&mut self, tick_rate: u32) -> bool {
        self.time += TICKS_PER_SECOND / f64::from(tick_rate);
        let length = if self.day { DAY_LENGTH } else { NIGHT_LENGTH };
        if self.time < length {
            return false;
        }
        self.time -= length;
        self.day = !self.day;
        true
    }

    pub(super) fn packet(&self) -> Time {
        Time { day_time: self.day, time: self.time as i32, sun_mod_y: 0, moon_mod_y: 0 }
    }
}
