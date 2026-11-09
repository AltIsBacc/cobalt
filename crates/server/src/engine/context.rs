use cobalt_format::World;
use cobalt_protocol::registry::Disconnect;
use cobalt_protocol::{Frame, NetworkText, S2c};

use super::error::ClientError;
use super::event::GameEvent;
use super::module::StageKind;
use super::outbox::Outbox;
use super::state::State;
use crate::model::{ClientId, Event, Player};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Audience {
    Everyone,
    Except(ClientId),
    Only(ClientId),
}

impl Audience {
    const fn includes(self, id: ClientId) -> bool {
        match self {
            Self::Everyone => true,
            Self::Except(excluded) => excluded.get() != id.get(),
            Self::Only(target) => target.get() == id.get(),
        }
    }
}

pub struct Context<'a> {
    state: &'a mut State,
    out: &'a mut Outbox,
}

impl<'a> Context<'a> {
    pub(super) fn new(state: &'a mut State, out: &'a mut Outbox) -> Self {
        Self { state, out }
    }

    #[must_use]
    pub fn tick(&self) -> u64 {
        self.state.ticks
    }

    #[must_use]
    pub fn tick_rate(&self) -> u32 {
        self.state.tick_rate
    }

    #[must_use]
    pub fn world(&self) -> &World {
        &self.state.world
    }

    pub fn world_mut(&mut self) -> &mut World {
        &mut self.state.world
    }

    #[must_use]
    pub fn stage(&self, id: ClientId) -> Option<StageKind> {
        self.state.stage_kind(id)
    }

    #[must_use]
    pub fn player(&self, id: ClientId) -> Option<&Player> {
        self.state.player(id)
    }

    pub fn player_mut(&mut self, id: ClientId) -> Option<&mut Player> {
        self.state.player_mut(id)
    }

    pub fn players(&self) -> impl Iterator<Item = (ClientId, &Player)> {
        self.state.playing()
    }

    pub fn send(&mut self, to: ClientId, packet: S2c<'_>) -> Result<(), ClientError> {
        Ok(self.out.send(to, packet)?)
    }

    pub fn send_with_world(
        &mut self,
        to: ClientId,
        build: impl for<'w> FnOnce(&'w World) -> S2c<'w>,
    ) -> Result<(), ClientError> {
        Ok(self.out.send(to, build(&self.state.world))?)
    }

    pub fn broadcast_where(
        &mut self,
        packet: S2c<'_>,
        mut filter: impl FnMut(ClientId, &Player) -> bool,
    ) {
        let recipients =
            self.state.playing().filter(|&(id, player)| filter(id, player)).map(|(id, _)| id);
        self.out.broadcast(recipients, packet);
    }

    pub fn relay(&mut self, to: ClientId, frames: &[u8]) {
        self.out.relay(to, frames);
    }

    pub fn broadcast(&mut self, audience: Audience, packet: S2c<'_>) {
        self.out.broadcast(recipients(self.state, audience), packet);
    }

    pub fn forward(&mut self, audience: Audience, frame: Frame<'_>) -> Result<(), ClientError> {
        Ok(self.out.forward(recipients(self.state, audience), frame)?)
    }

    pub fn relay_to(&mut self, audience: Audience, frames: &[u8]) {
        self.out.relay_all(recipients(self.state, audience), frames);
    }

    pub fn emit(&mut self, event: GameEvent) {
        self.state.bus.emit(event);
    }

    pub(super) fn advance(&mut self, id: ClientId) {
        self.state.advance(id);
    }

    pub(super) fn publish(&mut self, event: Event) {
        self.out.emit(event);
    }

    pub fn disconnect(&mut self, id: ClientId, reason: &str) -> Result<(), ClientError> {
        self.out.send(id, Disconnect { reason: NetworkText::Literal(reason) }.into())?;
        self.state.close(id);
        Ok(())
    }
}

fn recipients(state: &State, audience: Audience) -> impl Iterator<Item = ClientId> {
    state.playing().map(|(id, _)| id).filter(move |&id| audience.includes(id))
}
