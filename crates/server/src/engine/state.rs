use std::time::Duration;

use cobalt_format::World;

use super::event::Bus;
use super::module::StageKind;
use super::session::Session;
use super::slots::Slots;
use crate::config::ServerConfig;
use crate::model::{ClientId, Player, Status};

pub(super) struct State {
    pub(super) world: World,
    pub(super) slots: Slots<Session>,
    pub(super) bus: Bus,
    pub(super) tick_rate: u32,
    pub(super) ticks: u64,
}

impl State {
    pub(super) fn new(world: World, config: &ServerConfig) -> Self {
        Self {
            world,
            slots: Slots::new(config.max_players),
            bus: Bus::default(),
            tick_rate: config.tick_rate,
            ticks: 0,
        }
    }

    pub(super) fn stage_kind(&self, id: ClientId) -> Option<StageKind> {
        let session = self.slots.get(id)?;
        (!session.closing).then_some(session.stage)
    }

    pub(super) fn is_closed(&self, id: ClientId) -> bool {
        self.slots.get(id).is_none_or(|session| session.closing)
    }

    pub(super) fn advance(&mut self, id: ClientId) {
        if let Some(session) = self.slots.get_mut(id).filter(|session| !session.closing) {
            session.stage = session.stage.next();
        }
    }

    pub(super) fn player(&self, id: ClientId) -> Option<&Player> {
        self.slots.get(id).filter(|session| !session.closing).map(|session| &session.player)
    }

    pub(super) fn player_mut(&mut self, id: ClientId) -> Option<&mut Player> {
        self.slots.get_mut(id).filter(|session| !session.closing).map(|session| &mut session.player)
    }

    pub(super) fn playing(&self) -> impl Iterator<Item = (ClientId, &Player)> {
        self.slots
            .iter()
            .filter(|(_, session)| session.is_playing())
            .map(|(id, session)| (id, &session.player))
    }

    pub(super) fn close(&mut self, id: ClientId) {
        if let Some(session) = self.slots.get_mut(id) {
            session.closing = true;
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
