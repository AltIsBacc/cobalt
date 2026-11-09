mod chat;
mod chest;
mod clock;
mod drops;
mod items;
mod login;
mod npcs;
mod roster;
mod sign;
mod terrain;
mod tiles;

use cobalt_format::World;

use super::{Group, Module, ModuleInfo};
use crate::config::ServerConfig;

pub(super) const LATE: i32 = 100;

const NAME: &str = "vanilla";
const DESCRIPTION: &str = "Vanilla Terraria Implementation";

pub(super) fn module(world: &World, config: &ServerConfig) -> Group {
    let info = ModuleInfo::new(NAME)
        .version(env!("CARGO_PKG_VERSION"))
        .authors(env!("CARGO_PKG_AUTHORS").split(':').filter(|author| !author.is_empty()))
        .description(DESCRIPTION);
    Group::new(info, submodules(world, config))
}

fn submodules(world: &World, config: &ServerConfig) -> Vec<Box<dyn Module>> {
    vec![
        Box::new(clock::ClockModule::new(world)),
        Box::new(terrain::TerrainModule::new(world, config)),
        Box::new(tiles::TilesModule),
        Box::new(chest::ChestModule),
        Box::new(sign::SignModule),
        Box::new(chat::ChatModule),
        Box::new(items::ItemsModule::new()),
        Box::new(login::LoginModule),
        Box::new(roster::RosterModule::new(config)),
        Box::new(npcs::NpcModule::new(world)),
    ]
}
