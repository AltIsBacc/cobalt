use cobalt_format::{World, world::npcs::TownNpc};

use crate::engine::schedule::System;


pub(super) struct NPCS {

}

impl NPCS {
    pub(super) fn new(world: &World) -> Self {
        Self {

        }
    }
}

pub(super) struct NpcSystem;

impl System for NpcSystem {
    fn update(&self, state: &mut super::State) {

    }

    fn sync(&self, state: &mut super::State, out: &mut super::Outbox) {

    }
}

