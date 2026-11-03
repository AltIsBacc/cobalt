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

pub(super) struct TickNPCS;

impl System for TickNPCS {
    fn run(&mut self, state: &mut super::State, out: &mut super::Outbox) {
        
    }
}

pub(super) struct SyncNPCS;

impl System for SyncNPCS {
    fn run(&mut self, state: &mut super::State, out: &mut super::Outbox) {
        
    }
}

