use std::path::Path;

use anyhow::Result;
use cobalt_format::World;
use cobalt_server::{Server, ServerConfig};

use crate::server::console::ServerConsole;

pub mod console;

pub async fn new(world_path: impl AsRef<Path>) -> Result<ServerConsole> {
    let world = World::load(world_path)?;
    let running = Server::spawn(world, ServerConfig::default())?;

    Ok(ServerConsole::new(running))
}
