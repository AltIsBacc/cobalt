use cobalt_format::World;

use crate::engine::Module;

pub(super) struct NpcModule;

impl NpcModule {
    pub(super) fn new(_world: &World) -> Self {
        Self
    }
}

impl Module for NpcModule {
    fn name(&self) -> &'static str {
        "npcs"
    }
}
