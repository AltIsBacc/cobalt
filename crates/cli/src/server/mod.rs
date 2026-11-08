use std::net::SocketAddr;
use std::path::Path;

use anyhow::Result;
use cobalt_format::World;
use cobalt_server::{Server, ServerConfig};

use crate::server::console::ServerConsole;

pub mod console;

pub async fn new(
    world_path: impl AsRef<Path>,
    listen: SocketAddr,
    engine_core: Option<usize>,
) -> Result<ServerConsole> {
    let world = World::load(world_path)?;
    let config = ServerConfig { listen: Some(listen), engine_core, ..ServerConfig::default() };
    let running = Server::spawn(world, config)?;

    Ok(ServerConsole::new(running))
}
