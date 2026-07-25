use anyhow::Result;
use inquire::InquireError;
use std::path::Path;

mod server;

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
    options.push("n. New World".to_string());
    options.push("d. Delete World".to_string());

    let selection = inquire::Select::new("Choose a world or action:", options).prompt()?;

    if selection == "n. New World" {
        Ok(WorldAction::Create)
    } else if selection == "d. Delete World" {
        Ok(WorldAction::Delete)
    } else {
        let name = Path::new(&selection)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Cobalt World")
            .to_string();

        Ok(WorldAction::Select { name, path: selection })
    }
}

fn prompt_create_world() -> Result<String, InquireError> {
    inquire::Text::new("Enter new world name:").prompt()
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut worlds = vec![
        "/data/data/com.termux/files/home/Programs/TShock/Data/worlds/joaquinvincentdeveyra.wld".to_string()
    ];
    let mut console_instance = None;

    loop {
        print!("\x1B[2J\x1B[1H"); 

        match prompt_world_menu(&worlds) {
            Ok(WorldAction::Select { name, path }) => {
                println!("Loading world context '{}'...", name);
                match server::new(path).await {
                    Ok(console) => {
                        console_instance = Some(console);
                        break;
                    }
                    Err(e) => {
                        println!("Failed to instantiate game world server: {}", e);
                        std::thread::sleep(std::time::Duration::from_secs(2));
                    }
                }
            }
            Ok(WorldAction::Create) => {
                match prompt_create_world() {
                    Ok(new_name) => {
                        println!("Generating world architecture files for '{}'...", new_name);
                        worlds.push(format!("{}.wld", new_name));
                    }
                    Err(InquireError::OperationInterrupted) => continue,
                    Err(e) => return Err(e.into()),
                }
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
            Ok(WorldAction::Delete) => {
                println!("Deletion menu logic goes here...");
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
            Err(InquireError::OperationInterrupted) => {
                println!("\nExiting...");
                return Ok(());
            }
            Err(e) => return Err(e.into()),
        }
    }

    if let Some(console) = console_instance {
        console.start().await?;
    }

    Ok(())
}

