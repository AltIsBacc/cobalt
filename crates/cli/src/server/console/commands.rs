use clap::{Args, Parser, Subcommand};

#[derive(Debug, Args, Clone)]
pub struct StringPath {
    #[arg(short, long)]
    pub path: String,
}

#[derive(Debug, Subcommand, Clone)]
pub enum ConsoleCommand {
    Status,
    Tick { rate: u32 },
}

#[derive(Debug, Parser, Clone)]
pub struct Cli {
    #[clap(subcommand)]
    pub command: ConsoleCommand,
}

