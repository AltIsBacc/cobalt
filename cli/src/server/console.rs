use std::fmt::Debug;

use anyhow::Result;
use async_trait::async_trait;
use cobalt_common::utils::bi_channel::CobaltSender;
use cobalt_server::messages::{CobaltCommand, CobaltResponse};
use derive_more::derive::{Constructor, Deref, DerefMut};
use rustyrepl::{Repl, ReplCommandProcessor};

use crate::server::console::commands::{Cli, ConsoleCommand};

pub mod commands;

#[derive(Constructor, Deref, DerefMut)]
pub struct ServerConsole(CobaltSender<CobaltCommand, CobaltResponse>);

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

