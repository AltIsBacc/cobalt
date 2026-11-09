use cobalt_format::world::item::Item;
use cobalt_protocol::registry::{ActiveChest, OpenChest, SyncPlayerChestIndex, UpdateChestItem};
use cobalt_protocol::{C2s, ChestName, Packet};

use crate::engine::world_info::narrow;
use crate::engine::{
    Audience, ClientError, Context, Handled, Incoming, Module, Subscription, Verdict,
};
use crate::model::ClientId;

const CHEST_SLOTS: usize = 40;
const NO_CHEST: i16 = -1;

pub(super) struct ChestModule;

impl Module for ChestModule {
    fn name(&self) -> &'static str {
        "chests"
    }

    fn subscriptions(&self) -> Vec<Subscription> {
        vec![Subscription::playing([OpenChest::ID, ActiveChest::ID, UpdateChestItem::ID])]
    }

    fn packet(&mut self, cx: &mut Context<'_>, from: ClientId, incoming: &Incoming<'_>) -> Handled {
        match &incoming.packet {
            Some(C2s::OpenChest(request)) => open(cx, from, request)?,
            Some(C2s::ActiveChest(active)) => close(cx, from, active),
            Some(C2s::UpdateChestItem(update)) => update_item(cx, from, update),
            _ => return Ok(Verdict::Pass),
        }
        Ok(Verdict::Stop)
    }
}

fn open(cx: &mut Context<'_>, id: ClientId, request: &OpenChest) -> Result<(), ClientError> {
    let (x, y) = (i32::from(request.x), i32::from(request.y));
    let Some(index) = cx.world().chests.iter().position(|chest| chest.x == x && chest.y == y)
    else {
        return Ok(());
    };
    if cx.players().any(|(viewer, player)| viewer != id && player.chest == Some(index)) {
        return Ok(());
    }

    let chest_id = narrow(index);
    for slot in 0..CHEST_SLOTS {
        let item = cx.world().chests[index].items.get(slot).copied().flatten();
        cx.send(id, slot_packet(chest_id, slot, item).into())?;
    }
    let name = cx.world().chests[index].name.clone();
    let name = ChestName(Some(name.as_str()));
    cx.send(id, ActiveChest { chest: chest_id, x: request.x, y: request.y, name }.into())?;

    set_chest(cx, id, Some(index));
    Ok(())
}

fn close(cx: &mut Context<'_>, id: ClientId, active: &ActiveChest<'_>) {
    if active.chest == NO_CHEST && cx.player(id).is_some_and(|player| player.chest.is_some()) {
        set_chest(cx, id, None);
    }
}

fn update_item(cx: &mut Context<'_>, id: ClientId, update: &UpdateChestItem) {
    let Ok(index) = usize::try_from(update.chest_id) else { return };
    let slot = usize::from(update.slot);
    let opened = cx.player(id).is_some_and(|player| player.chest == Some(index));
    if !opened || slot >= CHEST_SLOTS {
        return;
    }

    let Some(chest) = cx.world_mut().chests.get_mut(index) else { return };
    if chest.items.len() < CHEST_SLOTS {
        chest.items.resize(CHEST_SLOTS, None);
    }
    chest.items[slot] = (update.stack > 0).then_some(Item {
        net_id: i32::from(update.item_net_id),
        prefix: update.prefix,
        stack: update.stack,
    });

    cx.broadcast_where(update.clone().into(), |viewer, player| {
        viewer != id && player.chest == Some(index)
    });
}

fn set_chest(cx: &mut Context<'_>, id: ClientId, chest: Option<usize>) {
    let Some(player) = cx.player_mut(id) else { return };
    player.chest = chest;

    let chest = chest.map_or(NO_CHEST, narrow);
    let sync = SyncPlayerChestIndex { player_id: id.get(), chest };
    cx.broadcast(Audience::Except(id), sync.into());
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
