use cobalt_protocol::Frame;
use cobalt_protocol::registry::{UpdateItemDrop, UpdateItemOwner};

use super::drops;
use super::error::ClientError;
use super::schedule::System;
use super::section::TILE_PIXELS;
use super::world_info::narrow;
use super::{Outbox, State};
use crate::model::ClientId;

const CAPACITY: usize = 400;
const NEW_ITEM: i16 = 400;
const NO_OWNER: u8 = 255;
const LIFETIME_SECONDS: u64 = 300;
const POP_VELOCITY: f32 = -2.0;

pub(super) struct Items {
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
    pub(super) fn new() -> Self {
        Self { slots: (0..CAPACITY).map(|_| None).collect() }
    }

    pub(super) fn snapshot(&self) -> impl Iterator<Item = (UpdateItemDrop, UpdateItemOwner)> {
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

impl State {
    pub(super) fn update_item_drop(
        &mut self,
        id: ClientId,
        update: &UpdateItemDrop,
        frame: Frame<'_>,
        out: &mut Outbox,
    ) -> Result<(), ClientError> {
        if update.item_net_id == 0 || update.stack <= 0 {
            self.remove_item(update.item_id, out);
        } else if update.item_id == NEW_ITEM {
            let index = self.items.insert(WorldItem::dropped(update, id, self.ticks));
            self.announce_item(index, out);
        } else if let Some(item) = self.items.get_mut(update.item_id) {
            item.update(update);
            out.forward(self.audience(Some(id)), frame)?;
        }
        Ok(())
    }

    pub(super) fn drop_block(
        &mut self,
        miner: ClientId,
        block: u16,
        x: i16,
        y: i16,
        out: &mut Outbox,
    ) {
        let Some(kind) = drops::block_item(block) else { return };
        let (x, y) = (f32::from(x) * TILE_PIXELS, f32::from(y) * TILE_PIXELS);
        let index = self.items.insert(WorldItem::spawned(kind, x, y, miner, self.ticks));
        self.announce_item(index, out);
    }

    pub(super) fn disown_items(&mut self, owner: ClientId, out: &mut Outbox) {
        for packet in self.items.disown(owner.get()) {
            out.broadcast(self.audience(None), packet.into());
        }
    }

    fn remove_item(&mut self, index: i16, out: &mut Outbox) {
        let Some(item) = self.items.take(index) else { return };
        out.broadcast(self.audience(None), item.vanished(index).into());
    }

    fn announce_item(&self, index: i16, out: &mut Outbox) {
        let Some(item) = self.items.get(index) else { return };
        out.broadcast(self.audience(None), item.packet(index).into());
        out.broadcast(self.audience(None), item.owner_packet(index).into());
    }
}

pub(super) struct ItemSystem;

impl System for ItemSystem {
    fn sync(&self, state: &mut State, out: &mut Outbox) {
        let lifetime = LIFETIME_SECONDS * u64::from(state.tick_rate);
        let Some(cutoff) = state.ticks.checked_sub(lifetime) else { return };
        for packet in state.items.expire(cutoff) {
            out.broadcast(state.audience(None), packet.into());
        }
    }
}
