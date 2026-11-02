mod error;
mod outbox;
mod slots;
mod stage;
mod world_info;

use std::mem;
use std::time::Duration;

use cobalt_format::World;
use cobalt_protocol::registry::{Disconnect, RequestEssentialTiles, SetUserSlot};
use cobalt_protocol::{Frame, Frames, NetworkText, Packet, ProtocolError, ProtocolResult};

use self::error::ClientError;
pub(crate) use self::outbox::Outbox;
use self::slots::Slots;
use self::stage::{Absorbed, Authenticating, Stage};
use crate::client::ClientId;
use crate::config::{self, ServerConfig};
use crate::error::ServerResult;
use crate::event::Event;
use crate::log::debug_log;
use crate::status::Status;

const SHUTTING_DOWN: &str = "the server is shutting down";
const FULL: &str = "the server is full";
const TILES_UNAVAILABLE: &str = "tile data is not available yet";

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
    world: World,
    slots: Slots<Stage>,
    tick_rate: u32,
    ticks: u64,
    open: bool,
}

impl Engine {
    pub(crate) fn new(world: World, config: &ServerConfig) -> Self {
        Self {
            world,
            slots: Slots::new(config.max_players),
            tick_rate: config.tick_rate,
            ticks: 0,
            open: true,
        }
    }

    pub(crate) const fn is_open(&self) -> bool {
        self.open
    }

    pub(crate) const fn is_empty(&self) -> bool {
        self.slots.len() == 0
    }

    pub(crate) fn close(&mut self) -> bool {
        mem::replace(&mut self.open, false)
    }

    pub(crate) const fn tick_rate(&self) -> u32 {
        self.tick_rate
    }

    pub(crate) fn tick(&mut self) -> u32 {
        self.ticks += 1;
        self.tick_rate
    }

    pub(crate) fn set_tick_rate(&mut self, rate: u32) -> ServerResult {
        config::validate_tick_rate(rate)?;
        self.tick_rate = rate;
        debug_log!("tick rate set to {rate}");
        Ok(())
    }

    pub(crate) fn status(&self, uptime: Duration) -> Status {
        Status {
            world_name: self.world.header.name.clone(),
            players: self.slots.len(),
            max_players: self.slots.capacity(),
            tick_rate: self.tick_rate,
            ticks: self.ticks,
            uptime,
        }
    }

    pub(crate) fn join(&mut self, out: &mut Outbox) -> ProtocolResult<Admission> {
        if !self.open {
            return Ok(Admission::Refused(SHUTTING_DOWN));
        }

        let admitted = self.slots.insert_with(|id| {
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

    pub(crate) fn receive(
        &mut self,
        id: ClientId,
        bytes: &[u8],
        out: &mut Outbox,
    ) -> Result<Flow, ClientError> {
        for frame in Frames::new(bytes) {
            if self.dispatch(id, frame?, out)? == Flow::Close {
                return Ok(Flow::Close);
            }
        }
        Ok(Flow::Open)
    }

    fn dispatch(
        &mut self,
        id: ClientId,
        frame: Frame<'_>,
        out: &mut Outbox,
    ) -> Result<Flow, ClientError> {
        debug_log!("[cid {id}] {} : {:?}", frame.kind, frame.payload);
        let Some(stage) = self.slots.get_mut(id) else { return Ok(Flow::Close) };

        match mem::replace(stage, Stage::Closing) {
            Stage::Authenticating(mut authenticating) => match authenticating.absorb(frame)? {
                Absorbed::Pending => {
                    *stage = Stage::Authenticating(authenticating);
                    Ok(Flow::Open)
                }
                Absorbed::Complete => match authenticating.finish() {
                    Ok(player) => {
                        debug_log!("{id} logged in as {}", player.name);
                        out.emit(Event::ClientLoggedIn { id });
                        out.send(id, world_info::build(&self.world).into())?;
                        *stage = Stage::Loading;
                        Ok(Flow::Open)
                    }
                    Err(reason) => {
                        out.send(id, refusal(reason).into())?;
                        Ok(Flow::Close)
                    }
                },
            },
            Stage::Loading => {
                *stage = Stage::Loading;
                if frame.kind != RequestEssentialTiles::ID {
                    return Ok(Flow::Open);
                }
                out.send(id, refusal(TILES_UNAVAILABLE).into())?;
                Ok(Flow::Close)
            }
            Stage::Closing => Ok(Flow::Close),
        }
    }

    pub(crate) fn release(&mut self, id: ClientId, out: &mut Outbox) {
        if self.slots.remove(id).is_some() {
            out.emit(Event::ClientLeft { id });
        }
    }
}

fn refusal(reason: &'static str) -> Disconnect<'static> {
    Disconnect { reason: NetworkText::Literal(reason) }
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
