mod chat;
mod chest;
mod clock;
mod dispatch;
mod drops;
mod error;
mod items;
mod npcs;
mod outbox;
mod presence;
mod schedule;
mod section;
mod sign;
mod slots;
mod stage;
mod state;
mod tiles;
mod world_info;

use std::mem;
use std::time::Duration;

use cobalt_format::World;
use cobalt_protocol::registry::{PlayerActive, SetUserSlot};
use cobalt_protocol::{ProtocolError, ProtocolResult};

pub(crate) use self::outbox::Outbox;
use self::stage::{Authenticating, Stage};
use self::state::State;
use crate::config::{self, ServerConfig};
use crate::error::ServerResult;
use crate::log::debug_log;
use crate::model::{ClientId, Event, Status};

const SHUTTING_DOWN: &str = "the server is shutting down";
const FULL: &str = "the server is full";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Admission {
    Admitted(ClientId),
    Refused(&'static str),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Flow {
    Open,
    Close,
}

pub(crate) struct Engine {
    state: State,
    open: bool,
}

impl Engine {
    pub(crate) fn new(world: World, config: &ServerConfig) -> Self {
        Self { state: State::new(world, config), open: true }
    }

    pub(crate) const fn is_open(&self) -> bool {
        self.open
    }

    pub(crate) const fn is_empty(&self) -> bool {
        self.state.slots.len() == 0
    }

    pub(crate) fn close(&mut self) -> bool {
        mem::replace(&mut self.open, false)
    }

    pub(crate) const fn tick_rate(&self) -> u32 {
        self.state.tick_rate
    }

    pub(crate) fn tick(&mut self, out: &mut Outbox) -> u32 {
        self.state.ticks += 1;
        schedule::run(&mut self.state, out);
        self.state.tick_rate
    }

    pub(crate) fn set_tick_rate(&mut self, rate: u32) -> ServerResult {
        config::validate_tick_rate(rate)?;
        self.state.tick_rate = rate;
        debug_log!("tick rate set to {rate}");
        Ok(())
    }

    pub(crate) fn status(&self, uptime: Duration) -> Status {
        self.state.status(uptime)
    }

    pub(crate) fn join(&mut self, out: &mut Outbox) -> ProtocolResult<Admission> {
        if !self.open {
            return Ok(Admission::Refused(SHUTTING_DOWN));
        }

        let admitted = self.state.slots.insert_with(|id| {
            out.send(id, SetUserSlot { player_id: id.get() }.into())?;
            Ok::<_, ProtocolError>(Stage::Authenticating(Authenticating::new()))
        })?;

        Ok(match admitted {
            Some(id) => {
                out.emit(Event::ClientJoined { id });
                Admission::Admitted(id)
            }
            None => Admission::Refused(FULL),
        })
    }

    pub(crate) fn release(&mut self, id: ClientId, out: &mut Outbox) {
        let Some(stage) = self.state.slots.remove(id) else { return };
        if matches!(stage, Stage::Playing(_)) {
            self.state.disown_items(id, out);
            let gone = PlayerActive { player_id: id.get(), active: 0 };
            out.broadcast(self.state.audience(None), gone.into());
        }
        out.emit(Event::ClientLeft { id });
    }
}

#[cfg(test)]
mod tests {
    use cobalt_format::world::header::{Difficulty, Size};

    use super::*;

    fn engine(max_players: u8) -> Engine {
        let world =
            World::try_new("test", Size::try_new(4, 4).unwrap(), Difficulty::Normal).unwrap();
        Engine::new(world, &ServerConfig { max_players, ..ServerConfig::default() })
    }

    fn events(out: &mut Outbox) -> Vec<Event> {
        let mut events = Vec::new();
        out.drain(|_, _| {}, |event| events.push(event));
        events
    }

    #[test]
    fn clients_join_and_leave() {
        let mut engine = engine(2);
        let mut out = Outbox::new(2);

        let Admission::Admitted(a) = engine.join(&mut out).unwrap() else {
            panic!("first client was not admitted");
        };
        let Admission::Admitted(b) = engine.join(&mut out).unwrap() else {
            panic!("second client was not admitted");
        };
        assert_ne!(a, b);
        assert_eq!(engine.join(&mut out).unwrap(), Admission::Refused(FULL));

        engine.release(a, &mut out);
        engine.release(a, &mut out);
        assert_eq!(engine.status(Duration::ZERO).players, 1);
        assert_eq!(engine.join(&mut out).unwrap(), Admission::Admitted(a));

        assert_eq!(
            events(&mut out),
            [
                Event::ClientJoined { id: a },
                Event::ClientJoined { id: b },
                Event::ClientLeft { id: a },
                Event::ClientJoined { id: a },
            ]
        );
    }
}
