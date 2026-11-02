mod clock;
mod error;
mod outbox;
mod section;
mod slots;
mod stage;
mod world_info;

use std::mem;
use std::time::Duration;

use cobalt_format::World;
use cobalt_protocol::registry::{
    Disconnect, FinishedConnectingToServer, PlayerActive, PlayerControls, PlayerHp, PlayerInfo,
    PlayerInventorySlot, PlayerMana, PlayerTeam, RequestEssentialTiles, SetUserSlot, SpawnPlayer,
    UpdatePlayerBuff,
};
use cobalt_protocol::{
    Frame, Frames, HEADER_LEN, NetworkText, Packet, ProtocolError, ProtocolResult, S2c,
};

use self::clock::Clock;
use self::error::ClientError;
pub(crate) use self::outbox::Outbox;
use self::slots::Slots;
use self::stage::{Absorbed, Authenticating, Stage, remember};
use self::world_info::narrow;
use crate::client::ClientId;
use crate::config::{self, ServerConfig};
use crate::error::ServerResult;
use crate::event::Event;
use crate::log::debug_log;
use crate::player::{Item, Player};
use crate::status::Status;

const SHUTTING_DOWN: &str = "the server is shutting down";
const FULL: &str = "the server is full";
const RELAYED: [u8; 8] = [
    PlayerInfo::ID,
    PlayerInventorySlot::ID,
    SpawnPlayer::ID,
    PlayerControls::ID,
    PlayerHp::ID,
    PlayerMana::ID,
    PlayerTeam::ID,
    UpdatePlayerBuff::ID,
];

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
    clock: Clock,
    slots: Slots<Stage>,
    tick_rate: u32,
    ticks: u64,
    open: bool,
}

impl Engine {
    pub(crate) fn new(world: World, config: &ServerConfig) -> Self {
        Self {
            clock: Clock::new(&world),
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

    pub(crate) fn tick(&mut self, out: &mut Outbox) -> u32 {
        self.ticks += 1;
        if self.clock.advance(self.tick_rate) {
            self.broadcast(self.clock.packet().into(), None, out);
        }
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
                        *stage = Stage::Loading(player);
                        Ok(Flow::Open)
                    }
                    Err(reason) => {
                        out.send(id, refusal(reason).into())?;
                        Ok(Flow::Close)
                    }
                },
            },
            Stage::Loading(player) => {
                if frame.kind != RequestEssentialTiles::ID {
                    *stage = Stage::Loading(player);
                    return Ok(Flow::Open);
                }
                section::send_spawn(&self.world, id, out)?;
                *stage = Stage::Spawning(player);
                Ok(Flow::Open)
            }
            Stage::Spawning(player) => {
                if frame.kind != SpawnPlayer::ID {
                    *stage = Stage::Spawning(player);
                    return Ok(Flow::Open);
                }
                *stage = Stage::Playing(player);
                self.admit(id, frame, out)?;
                Ok(Flow::Open)
            }
            Stage::Playing(mut player) => {
                remember(&mut player, frame)?;
                *stage = Stage::Playing(player);
                if RELAYED.contains(&frame.kind) {
                    self.relay(&attributed(frame, id)?, Some(id), out);
                }
                Ok(Flow::Open)
            }
            Stage::Closing => Ok(Flow::Close),
        }
    }

    fn admit(&self, id: ClientId, spawn: Frame<'_>, out: &mut Outbox) -> Result<(), ClientError> {
        let Some(Stage::Playing(joiner)) = self.slots.get(id) else { return Ok(()) };

        for (other, player) in self.playing().filter(|&(other, _)| other != id) {
            describe(other, player, id, out)?;
        }
        out.send(id, self.clock.packet().into())?;
        out.send(id, FinishedConnectingToServer {}.into())?;

        let spawned = attributed(spawn, id)?;
        for (other, _) in self.playing().filter(|&(other, _)| other != id) {
            describe(id, joiner, other, out)?;
            out.relay(other, &spawned);
        }
        Ok(())
    }

    fn playing(&self) -> impl Iterator<Item = (ClientId, &Player)> {
        self.slots.iter().filter_map(|(id, stage)| match stage {
            Stage::Playing(player) => Some((id, player)),
            _ => None,
        })
    }

    fn relay(&self, bytes: &[u8], except: Option<ClientId>, out: &mut Outbox) {
        for (to, _) in self.playing().filter(|&(to, _)| Some(to) != except) {
            out.relay(to, bytes);
        }
    }

    fn broadcast(&self, packet: S2c<'_>, except: Option<ClientId>, out: &mut Outbox) {
        let mut bytes = Vec::new();
        match packet.write(&mut bytes) {
            Ok(()) => self.relay(&bytes, except, out),
            Err(error) => debug_log!("broadcast failed: {error}"),
        }
    }

    pub(crate) fn release(&mut self, id: ClientId, out: &mut Outbox) {
        let Some(stage) = self.slots.remove(id) else { return };
        if matches!(stage, Stage::Playing(_)) {
            self.broadcast(PlayerActive { player_id: id.get(), active: 0 }.into(), None, out);
        }
        out.emit(Event::ClientLeft { id });
    }
}

fn attributed(frame: Frame<'_>, from: ClientId) -> ProtocolResult<Vec<u8>> {
    let mut bytes = Vec::with_capacity(HEADER_LEN + frame.payload.len());
    frame.write(&mut bytes)?;
    if let Some(owner) = bytes.get_mut(HEADER_LEN) {
        *owner = from.get();
    }
    Ok(bytes)
}

fn describe(
    owner: ClientId,
    player: &Player,
    to: ClientId,
    out: &mut Outbox,
) -> Result<(), ClientError> {
    let player_id = owner.get();

    out.send(to, PlayerActive { player_id, active: 1 }.into())?;
    if !player.appearance.is_empty() {
        let info = Frame { kind: PlayerInfo::ID, payload: &player.appearance };
        out.relay(to, &attributed(info, owner)?);
    }
    out.send(to, PlayerHp { player_id, hp: player.hp, max_hp: player.max_hp }.into())?;
    out.send(to, PlayerMana { player_id, mana: player.mana, max_mana: player.max_mana }.into())?;
    out.send(to, PlayerTeam { player_id, team: player.team }.into())?;
    out.send(to, UpdatePlayerBuff { player_id, buffs: player.buffs }.into())?;

    let held = player.inventory.iter().enumerate().filter(|(_, item)| **item != Item::default());
    for (slot, item) in held {
        let slot = PlayerInventorySlot {
            player_id,
            slot: narrow(slot),
            stack: item.stack,
            prefix: item.prefix,
            item_net_id: item.net_id,
        };
        out.send(to, slot.into())?;
    }
    Ok(())
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
