use std::fmt::Debug;

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
        let mut repl = Repl::<Cli>::new(Box::new(self), None, Some(">> ".to_string()))?;
        repl.process().await
    }
}

impl Debug for ServerConsole {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { Ok(()) }
}

#[async_trait]
impl ReplCommandProcessor<Cli> for ServerConsole {
    fn is_quit(&self, command: &str) -> bool {
        matches!(command, "exit-nosave" | "q")
    }

    async fn process_command(&self, command: Cli) -> Result<()> {
        match command.command {
            ConsoleCommand::Exit => {
                
            }
        };

        Ok(())
    }
}

