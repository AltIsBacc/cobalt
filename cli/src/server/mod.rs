use std::path::Path;

use anyhow::Result;
use cobalt_format::world::loader::WorldLoader;
use cobalt_server::CobaltServer;

use crate::server::{console::ServerConsole};

pub mod console;

pub async fn new<S: AsRef<Path> + Send + 'static>(world_path: S) -> Result<ServerConsole> {
    let handle = CobaltServer::spawn(
        WorldLoader(world_path),
        64, 3, /* increase if slow? */
        60
    )?;

    Ok(ServerConsole::new(handle.commands))
}

