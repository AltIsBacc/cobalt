mod context;
mod error;
mod event;
mod group;
mod module;
mod modules;
mod outbox;
mod router;
mod session;
mod slots;
mod state;
mod vanilla;
mod world_info;

use std::mem;
use std::time::Duration;

use cobalt_format::World;
use cobalt_protocol::registry::SetUserSlot;
use cobalt_protocol::{Frames, ProtocolError, ProtocolResult};

pub use self::context::{Audience, Context};
pub use self::error::ClientError;
pub use self::event::GameEvent;
pub use self::group::Group;
pub use self::module::{Handled, Incoming, Module, ModuleInfo, StageKind, Subscription, Verdict};
use self::modules::Modules;
pub(crate) use self::outbox::Outbox;
use self::session::Session;
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
    modules: Modules,
    open: bool,
}

impl Engine {
    pub(crate) fn new(world: World, config: &ServerConfig) -> Self {
        Self::with_modules(world, config, Vec::new())
    }

    pub(crate) fn with_modules(
        world: World,
        config: &ServerConfig,
        modules: Vec<Box<dyn Module>>,
    ) -> Self {
        let mut builtin: Vec<Box<dyn Module>> = vec![Box::new(vanilla::module(&world, config))];
        builtin.extend(modules);
        Self { state: State::new(world, config), modules: Modules::new(builtin), open: true }
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

    pub(crate) fn start(&mut self, out: &mut Outbox) {
        self.modules.start(&mut self.state, out);
        self.modules.settle(&mut self.state, out);
    }

    pub(crate) fn stop(&mut self, out: &mut Outbox) {
        self.modules.stop(&mut self.state, out);
        self.modules.settle(&mut self.state, out);
    }

    pub(crate) fn tick(&mut self, out: &mut Outbox) -> u32 {
        self.state.ticks += 1;
        self.modules.update(&mut self.state, out);
        self.modules.sync(&mut self.state, out);
        self.modules.settle(&mut self.state, out);
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
            Ok::<_, ProtocolError>(Session::new())
        })?;

        Ok(match admitted {
            Some(id) => {
                out.emit(Event::ClientJoined { id });
                Admission::Admitted(id)
            }
            None => Admission::Refused(FULL),
        })
    }

    pub(crate) fn receive(
        &mut self,
        id: ClientId,
        bytes: &[u8],
        out: &mut Outbox,
    ) -> Result<Flow, ClientError> {
        for frame in Frames::new(bytes) {
            let frame = frame?;
            debug_log!("[cid {id}] {} : {:?}", frame.kind, frame.payload);
            self.modules.dispatch(&mut self.state, out, id, frame)?;
            self.modules.settle(&mut self.state, out);
            if self.state.is_closed(id) {
                return Ok(Flow::Close);
            }
        }
        Ok(Flow::Open)
    }

    pub(crate) fn release(&mut self, id: ClientId, out: &mut Outbox) {
        let Some(session) = self.state.slots.remove(id) else { return };
        if session.stage == StageKind::Playing {
            self.state.bus.emit(GameEvent::PlayerLeft { id });
        }
        out.emit(Event::ClientLeft { id });
        self.modules.settle(&mut self.state, out);
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
