use cobalt_protocol::C2sPacket;

use super::error::{SessionError, SessionResult};
use crate::player::{Item, Player};

pub(super) fn absorb(player: &mut Player, packet: C2sPacket) -> SessionResult {
    match packet {
        C2sPacket::PlayerInfo { name, .. } => player.name = name,
        C2sPacket::ClientUuid { uuid } => player.uuid = uuid,
        C2sPacket::PlayerHp { hp, max_hp, .. } => (player.hp, player.max_hp) = (hp, max_hp),
        C2sPacket::PlayerMana { mana, max_mana, .. } => {
            (player.mana, player.max_mana) = (mana, max_mana);
        }
        C2sPacket::UpdatePlayerBuff { buffs, .. } => player.buffs = buffs,
        C2sPacket::PlayerInventorySlot { slot, stack, prefix, item_net_id, .. } => {
            let item = usize::try_from(slot)
                .ok()
                .and_then(|index| player.inventory.get_mut(index))
                .ok_or(SessionError::InvalidSlot(slot))?;
            *item = Item { stack, prefix, net_id: item_net_id };
        }
        other => return Err(SessionError::UnexpectedPacket(other.message_type())),
    }
    Ok(())
}
