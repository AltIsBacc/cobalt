use cobalt_protocol::{C2s, Frame};

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
        let Some(packet) = C2s::decode(frame)? else { return Ok(Absorbed::Pending) };

        match packet {
            C2s::PlayerInfo(info) => player.name = info.name.to_owned(),
            C2s::ClientUuid(client) => player.uuid = client.uuid.to_owned(),
            C2s::PlayerHp(hp) => (player.hp, player.max_hp) = (hp.hp, hp.max_hp),
            C2s::PlayerMana(mana) => (player.mana, player.max_mana) = (mana.mana, mana.max_mana),
            C2s::UpdatePlayerBuff(buffs) => player.buffs = buffs.buffs,
            C2s::PlayerInventorySlot(slot) => {
                let item = usize::try_from(slot.slot)
                    .ok()
                    .and_then(|index| player.inventory.get_mut(index))
                    .ok_or(ClientError::InvalidSlot(slot.slot))?;
                *item = Item { stack: slot.stack, prefix: slot.prefix, net_id: slot.item_net_id };
            }
            C2s::RequestWorldData(_) => return Ok(Absorbed::Complete),
            _ => {}
        }
        Ok(Absorbed::Pending)
    }

    pub(super) fn finish(self) -> Result<Player, &'static str> {
        if self.player.has_valid_name() { Ok(self.player) } else { Err("invalid player name") }
    }
}
