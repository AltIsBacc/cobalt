use std::path::Path;

use anyhow::Result;
use cobalt_format::world::loader::WorldLoader;
use cobalt_server::CobaltServer;

use crate::server::{console::ServerConsole};

pub mod console;

pub async fn new<S: AsRef<Path>>(world_path: S) -> Result<ServerConsole> {
    let (server, channels) = CobaltServer::new(
        WorldLoader(world_path),
        64, 3, /* increase if slow? */
        60
    )?;

    tokio::spawn(server.run_loop());

    Ok(ServerConsole::new(channels.commands))
}

