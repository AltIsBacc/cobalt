use cobalt_protocol::registry::{
    FinishedConnectingToServer, PlayerActive, PlayerHp, PlayerInfo, PlayerInventorySlot, PlayerMana,
    PlayerTeam, UpdatePlayerBuff,
};
use cobalt_protocol::{Frame, HEADER_LEN, Packet, ProtocolResult};

use super::error::ClientError;
use super::stage::Stage;
use super::world_info::narrow;
use super::{Outbox, State};
use crate::model::{ClientId, Item, Player};

impl State {
    pub(super) fn admit(
        &self,
        id: ClientId,
        spawn: Frame<'_>,
        out: &mut Outbox,
    ) -> Result<(), ClientError> {
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

    pub(super) fn player(&self, id: ClientId) -> Option<&Player> {
        match self.slots.get(id)? {
            Stage::Playing(player) => Some(player),
            _ => None,
        }
    }

    pub(super) fn player_mut(&mut self, id: ClientId) -> Option<&mut Player> {
        match self.slots.get_mut(id)? {
            Stage::Playing(player) => Some(player),
            _ => None,
        }
    }

    pub(super) fn playing(&self) -> impl Iterator<Item = (ClientId, &Player)> {
        self.slots.iter().filter_map(|(id, stage)| match stage {
            Stage::Playing(player) => Some((id, player)),
            _ => None,
        })
    }

    pub(super) fn audience(&self, except: Option<ClientId>) -> impl Iterator<Item = ClientId> {
        self.playing().map(|(id, _)| id).filter(move |&id| Some(id) != except)
    }
}

pub(super) fn attributed(frame: Frame<'_>, from: ClientId) -> ProtocolResult<Vec<u8>> {
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
