use cobalt_protocol::c2s::{ClientUuid, RequestWorldData};
use cobalt_protocol::shared::{
    PlayerHp, PlayerInfo, PlayerInventorySlot, PlayerMana, UpdatePlayerBuff,
};
use cobalt_protocol::{Frame, Packet};

use super::error::ClientError;
use crate::player::{Item, Player};

pub(super) enum Stage {
    Authenticating(Authenticating),
    Loading,
    Closing,
}

pub(super) enum Absorbed {
    Pending,
    Complete,
}

pub(super) struct Authenticating {
    player: Player,
}

impl Authenticating {
    pub(super) fn new() -> Self {
        Self { player: Player::default() }
    }

    pub(super) fn absorb(&mut self, frame: Frame<'_>) -> Result<Absorbed, ClientError> {
        let player = &mut self.player;
        match frame.kind {
            PlayerInfo::ID => player.name = frame.decode_c2s::<PlayerInfo>()?.name.to_owned(),
            ClientUuid::ID => player.uuid = frame.decode_c2s::<ClientUuid>()?.uuid.to_owned(),
            PlayerHp::ID => {
                let packet = frame.decode_c2s::<PlayerHp>()?;
                (player.hp, player.max_hp) = (packet.hp, packet.max_hp);
            }
            PlayerMana::ID => {
                let packet = frame.decode_c2s::<PlayerMana>()?;
                (player.mana, player.max_mana) = (packet.mana, packet.max_mana);
            }
            UpdatePlayerBuff::ID => player.buffs = frame.decode_c2s::<UpdatePlayerBuff>()?.buffs,
            PlayerInventorySlot::ID => {
                let packet = frame.decode_c2s::<PlayerInventorySlot>()?;
                let item = usize::try_from(packet.slot)
                    .ok()
                    .and_then(|index| player.inventory.get_mut(index))
                    .ok_or(ClientError::InvalidSlot(packet.slot))?;
                *item =
                    Item { stack: packet.stack, prefix: packet.prefix, net_id: packet.item_net_id };
            }
            RequestWorldData::ID => return Ok(Absorbed::Complete),
            _ => {}
        }
        Ok(Absorbed::Pending)
    }

    pub(super) fn finish(self) -> Result<Player, &'static str> {
        if self.player.has_valid_name() { Ok(self.player) } else { Err("invalid player name") }
    }
}
