use std::fmt;

use anyhow::Result;
use async_trait::async_trait;
use cobalt_server::RunningServer;
use derive_more::derive::{Constructor, Deref, DerefMut};
use rustyrepl::{Repl, ReplCommandProcessor};

use crate::server::console::commands::{Cli, ConsoleCommand};

pub mod commands;

#[derive(Constructor, Deref, DerefMut)]
pub struct ServerConsole(RunningServer);

impl ServerConsole {
    pub async fn start(self) -> Result<()> {
        if let Some(addr) = self.local_addr() {
            println!("Listening on {addr}");
        }

        let handle = self.handle().clone();
        let mut repl = Repl::<Cli>::new(Box::new(self), None, Some(">> ".to_owned()))?;
        repl.process().await?;

        handle.shutdown().await?;
        Ok(())
    }
}

impl fmt::Debug for ServerConsole {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Ok(())
    }
}

#[async_trait]
impl ReplCommandProcessor<Cli> for ServerConsole {
    fn is_quit(&self, command: &str) -> bool {
        matches!(command, "exit" | "exit-nosave" | "q")
    }

    async fn process_command(&self, command: Cli) -> Result<()> {
        match command.command {
            ConsoleCommand::Status => {
                let status = self.handle().status().await?;
                println!(
                    "{}: {}/{} players, {} tps, {} ticks, up {:.0?}",
                    status.world_name,
                    status.players,
                    status.max_players,
                    status.tick_rate,
                    status.ticks,
                    status.uptime,
                );
            }
            ConsoleCommand::Tick { rate } => self.handle().set_tick_rate(rate).await?,
        }

        Ok(())
    }
}
