use cobalt_protocol::registry::{UpdateItemDrop, UpdateItemOwner};
use cobalt_protocol::{C2s, Frame, Packet};

use super::drops;
use super::terrain::TILE_PIXELS;
use crate::engine::world_info::narrow;
use crate::engine::{
    Audience, ClientError, Context, GameEvent, Handled, Incoming, Module, Subscription, Verdict,
};
use crate::model::ClientId;

const CAPACITY: usize = 400;
const NEW_ITEM: i16 = 400;
const NO_OWNER: u8 = 255;
const LIFETIME_SECONDS: u64 = 300;
const POP_VELOCITY: f32 = -2.0;

struct Items {
    slots: Box<[Option<WorldItem>]>,
}

struct WorldItem {
    x: f32,
    y: f32,
    velocity_x: f32,
    velocity_y: f32,
    stack: i16,
    prefix: u8,
    kind: i16,
    owner: u8,
    born: u64,
}

impl Items {
    fn new() -> Self {
        Self { slots: (0..CAPACITY).map(|_| None).collect() }
    }

    fn snapshot(&self) -> impl Iterator<Item = (UpdateItemDrop, UpdateItemOwner)> {
        self.slots.iter().enumerate().filter_map(|(index, slot)| {
            let item = slot.as_ref()?;
            let index = narrow(index);
            Some((item.packet(index), item.owner_packet(index)))
        })
    }

    fn expire(&mut self, cutoff: u64) -> Vec<UpdateItemDrop> {
        self.slots
            .iter_mut()
            .enumerate()
            .filter_map(|(index, slot)| {
                let item = slot.take_if(|item| item.born <= cutoff)?;
                Some(item.vanished(narrow(index)))
            })
            .collect()
    }

    fn disown(&mut self, owner: u8) -> Vec<UpdateItemOwner> {
        self.slots
            .iter_mut()
            .enumerate()
            .filter_map(|(index, slot)| {
                let item = slot.as_mut().filter(|item| item.owner == owner)?;
                item.owner = NO_OWNER;
                Some(item.owner_packet(narrow(index)))
            })
            .collect()
    }

    fn insert(&mut self, item: WorldItem) -> i16 {
        let index = self.slots.iter().position(Option::is_none).unwrap_or_else(|| self.oldest());
        self.slots[index] = Some(item);
        narrow(index)
    }

    fn oldest(&self) -> usize {
        self.slots
            .iter()
            .enumerate()
            .min_by_key(|(_, slot)| slot.as_ref().map(|item| item.born))
            .map_or(0, |(index, _)| index)
    }

    fn get(&self, index: i16) -> Option<&WorldItem> {
        self.slots.get(usize::try_from(index).ok()?)?.as_ref()
    }

    fn get_mut(&mut self, index: i16) -> Option<&mut WorldItem> {
        self.slots.get_mut(usize::try_from(index).ok()?)?.as_mut()
    }

    fn take(&mut self, index: i16) -> Option<WorldItem> {
        self.slots.get_mut(usize::try_from(index).ok()?)?.take()
    }
}

impl WorldItem {
    fn dropped(update: &UpdateItemDrop, owner: ClientId, born: u64) -> Self {
        Self {
            x: update.x,
            y: update.y,
            velocity_x: update.velocity_x,
            velocity_y: update.velocity_y,
            stack: update.stack,
            prefix: update.prefix,
            kind: update.item_net_id,
            owner: owner.get(),
            born,
        }
    }

    fn spawned(kind: i16, x: f32, y: f32, owner: ClientId, born: u64) -> Self {
        Self {
            x,
            y,
            velocity_x: 0.0,
            velocity_y: POP_VELOCITY,
            stack: 1,
            prefix: 0,
            kind,
            owner: owner.get(),
            born,
        }
    }

    fn update(&mut self, update: &UpdateItemDrop) {
        self.x = update.x;
        self.y = update.y;
        self.velocity_x = update.velocity_x;
        self.velocity_y = update.velocity_y;
        self.stack = update.stack;
        self.prefix = update.prefix;
        self.kind = update.item_net_id;
    }

    fn packet(&self, index: i16) -> UpdateItemDrop {
        UpdateItemDrop {
            item_id: index,
            x: self.x,
            y: self.y,
            velocity_x: self.velocity_x,
            velocity_y: self.velocity_y,
            stack: self.stack,
            prefix: self.prefix,
            no_delay: 0,
            item_net_id: self.kind,
        }
    }

    fn owner_packet(&self, index: i16) -> UpdateItemOwner {
        UpdateItemOwner { item_id: index, player_id: self.owner }
    }

    fn vanished(&self, index: i16) -> UpdateItemDrop {
        UpdateItemDrop { stack: 0, prefix: 0, item_net_id: 0, ..self.packet(index) }
    }
}

pub(super) struct ItemsModule {
    items: Items,
}

impl ItemsModule {
    pub(super) fn new() -> Self {
        Self { items: Items::new() }
    }

    fn update_item_drop(
        &mut self,
        cx: &mut Context<'_>,
        id: ClientId,
        update: &UpdateItemDrop,
        frame: Frame<'_>,
    ) -> Result<(), ClientError> {
        if update.item_net_id == 0 || update.stack <= 0 {
            self.remove(cx, update.item_id);
        } else if update.item_id == NEW_ITEM {
            let index = self.items.insert(WorldItem::dropped(update, id, cx.tick()));
            self.announce(cx, index);
        } else if let Some(item) = self.items.get_mut(update.item_id) {
            item.update(update);
            cx.forward(Audience::Except(id), frame)?;
        }
        Ok(())
    }

    fn drop_block(&mut self, cx: &mut Context<'_>, miner: ClientId, block: u16, x: i16, y: i16) {
        let Some(kind) = drops::block_item(block) else { return };
        let (x, y) = (f32::from(x) * TILE_PIXELS, f32::from(y) * TILE_PIXELS);
        let index = self.items.insert(WorldItem::spawned(kind, x, y, miner, cx.tick()));
        self.announce(cx, index);
    }

    fn disown(&mut self, cx: &mut Context<'_>, owner: ClientId) {
        for packet in self.items.disown(owner.get()) {
            cx.broadcast(Audience::Everyone, packet.into());
        }
    }

    fn remove(&mut self, cx: &mut Context<'_>, index: i16) {
        let Some(item) = self.items.take(index) else { return };
        cx.broadcast(Audience::Everyone, item.vanished(index).into());
    }

    fn announce(&self, cx: &mut Context<'_>, index: i16) {
        let Some(item) = self.items.get(index) else { return };
        cx.broadcast(Audience::Everyone, item.packet(index).into());
        cx.broadcast(Audience::Everyone, item.owner_packet(index).into());
    }
}

impl Module for ItemsModule {
    fn name(&self) -> &'static str {
        "items"
    }

    fn subscriptions(&self) -> Vec<Subscription> {
        vec![Subscription::playing([UpdateItemDrop::ID])]
    }

    fn packet(&mut self, cx: &mut Context<'_>, from: ClientId, incoming: &Incoming<'_>) -> Handled {
        let Some(C2s::UpdateItemDrop(update)) = &incoming.packet else { return Ok(Verdict::Pass) };
        self.update_item_drop(cx, from, update, incoming.frame)?;
        Ok(Verdict::Stop)
    }

    fn event(&mut self, cx: &mut Context<'_>, event: &GameEvent) -> Result<(), ClientError> {
        match event {
            GameEvent::PlayerEntering { id } => {
                for (item, owner) in self.items.snapshot() {
                    cx.send(*id, item.into())?;
                    cx.send(*id, owner.into())?;
                }
            }
            GameEvent::PlayerLeft { id } => self.disown(cx, *id),
            GameEvent::TileBroken { by, block, x, y } => self.drop_block(cx, *by, *block, *x, *y),
            _ => {}
        }
        Ok(())
    }

    fn sync(&mut self, cx: &mut Context<'_>) {
        let lifetime = LIFETIME_SECONDS * u64::from(cx.tick_rate());
        let Some(cutoff) = cx.tick().checked_sub(lifetime) else { return };
        for packet in self.items.expire(cutoff) {
            cx.broadcast(Audience::Everyone, packet.into());
        }
    }
}
