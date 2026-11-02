use cobalt_format::world::item::Item;
use cobalt_protocol::ChestName;
use cobalt_protocol::registry::{ActiveChest, OpenChest, SyncPlayerChestIndex, UpdateChestItem};

use super::error::ClientError;
use super::world_info::narrow;
use super::{Engine, Outbox};
use crate::model::ClientId;

const CHEST_SLOTS: usize = 40;
const NO_CHEST: i16 = -1;

impl Engine {
    pub(super) fn open_chest(
        &mut self,
        id: ClientId,
        request: &OpenChest,
        out: &mut Outbox,
    ) -> Result<(), ClientError> {
        let (x, y) = (i32::from(request.x), i32::from(request.y));
        let Some(index) = self.world.chests.iter().position(|chest| chest.x == x && chest.y == y)
        else {
            return Ok(());
        };
        if self.chest_viewers(index).any(|viewer| viewer != id) {
            return Ok(());
        }

        let chest = &self.world.chests[index];
        let chest_id = narrow(index);
        for slot in 0..CHEST_SLOTS {
            let item = chest.items.get(slot).copied().flatten();
            out.send(id, slot_packet(chest_id, slot, item).into())?;
        }
        let name = ChestName(Some(chest.name.as_str()));
        out.send(id, ActiveChest { chest: chest_id, x: request.x, y: request.y, name }.into())?;

        self.set_chest(id, Some(index), out);
        Ok(())
    }

    pub(super) fn close_chest(&mut self, id: ClientId, active: &ActiveChest<'_>, out: &mut Outbox) {
        if active.chest == NO_CHEST
            && self.player(id).is_some_and(|player| player.chest.is_some())
        {
            self.set_chest(id, None, out);
        }
    }

    pub(super) fn update_chest_item(
        &mut self,
        id: ClientId,
        update: &UpdateChestItem,
        out: &mut Outbox,
    ) -> Result<(), ClientError> {
        let Ok(index) = usize::try_from(update.chest_id) else { return Ok(()) };
        let slot = usize::from(update.slot);
        let opened = self.player(id).is_some_and(|player| player.chest == Some(index));
        if !opened || slot >= CHEST_SLOTS {
            return Ok(());
        }

        let Some(chest) = self.world.chests.get_mut(index) else { return Ok(()) };
        if chest.items.len() < CHEST_SLOTS {
            chest.items.resize(CHEST_SLOTS, None);
        }
        chest.items[slot] = (update.stack > 0).then_some(Item {
            net_id: i32::from(update.item_net_id),
            prefix: update.prefix,
            stack: update.stack,
        });

        for viewer in self.chest_viewers(index).filter(|&viewer| viewer != id) {
            out.send(viewer, update.clone().into())?;
        }
        Ok(())
    }

    fn chest_viewers(&self, index: usize) -> impl Iterator<Item = ClientId> {
        self.playing()
            .filter(move |&(_, player)| player.chest == Some(index))
            .map(|(viewer, _)| viewer)
    }

    fn set_chest(&mut self, id: ClientId, chest: Option<usize>, out: &mut Outbox) {
        let Some(player) = self.player_mut(id) else { return };
        player.chest = chest;

        let chest = chest.map_or(NO_CHEST, narrow);
        let sync = SyncPlayerChestIndex { player_id: id.get(), chest };
        self.broadcast(sync.into(), Some(id), out);
    }
}

fn slot_packet(chest_id: i16, slot: usize, item: Option<Item>) -> UpdateChestItem {
    UpdateChestItem {
        chest_id,
        slot: narrow(slot),
        stack: item.map_or(0, |item| item.stack),
        prefix: item.map_or(0, |item| item.prefix),
        item_net_id: item.map_or(0, |item| narrow(item.net_id)),
    }
}
