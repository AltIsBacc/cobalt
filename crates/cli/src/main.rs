use std::net::SocketAddr;
use std::path::Path;
use std::thread;
use std::time::Duration;

use anyhow::Result;
use clap::Parser;
use inquire::InquireError;

mod server;

const NEW_WORLD: &str = "n. New World";
const DELETE_WORLD: &str = "d. Delete World";

#[derive(Debug, Parser)]
struct Args {
    #[arg(short, long, default_value_t = 7777)]
    port: u16,
}

#[derive(Debug, Clone)]
enum WorldAction {
    Select { name: String, path: String },
    Create,
    Delete,
}

fn prompt_world_menu(existing_worlds: &[String]) -> Result<WorldAction, InquireError> {
    println!("Cobalt - Server Console");
    println!("============================\n");

    let mut options = existing_worlds.to_vec();
    options.push(NEW_WORLD.to_owned());
    options.push(DELETE_WORLD.to_owned());

    let selection = inquire::Select::new("Choose a world or action:", options).prompt()?;

    match selection.as_str() {
        NEW_WORLD => Ok(WorldAction::Create),
        DELETE_WORLD => Ok(WorldAction::Delete),
        _ => {
            let name = Path::new(&selection)
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or("Cobalt World")
                .to_owned();

            Ok(WorldAction::Select { name, path: selection })
        }
    }
}

fn prompt_create_world() -> Result<String, InquireError> {
    inquire::Text::new("Enter new world name:").prompt()
}

#[tokio::main]
async fn main() -> Result<()> {
    let listen = SocketAddr::from(([0, 0, 0, 0], Args::parse().port));
    let mut worlds = vec![
        "/data/data/com.termux/files/home/Programs/TShock/Data/worlds/joaquinvincentdeveyra.wld".to_owned()
    ];

    let console = loop {
        print!("\x1B[2J\x1B[1H");

        match prompt_world_menu(&worlds) {
            Ok(WorldAction::Select { name, path }) => {
                println!("Loading world context '{name}'...");
                match server::new(path, listen).await {
                    Ok(console) => break console,
                    Err(error) => {
                        println!("Failed to instantiate game world server: {error}");
                        thread::sleep(Duration::from_secs(2));
                    }
                }
            }
            Ok(WorldAction::Create) => {
                match prompt_create_world() {
                    Ok(new_name) => {
                        println!("Generating world architecture files for '{new_name}'...");
                        worlds.push(format!("{new_name}.wld"));
                    }
                    Err(InquireError::OperationInterrupted) => continue,
                    Err(error) => return Err(error.into()),
                }
                thread::sleep(Duration::from_secs(1));
            }
            Ok(WorldAction::Delete) => {
                println!("Deletion menu logic goes here...");
                thread::sleep(Duration::from_secs(1));
            }
            Err(InquireError::OperationInterrupted) => {
                println!("\nExiting...");
                return Ok(());
            }
            Err(error) => return Err(error.into()),
        }
    };

    console.start().await?;
    Ok(())
}
