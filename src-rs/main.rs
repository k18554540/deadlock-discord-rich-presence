
use deadlock_discord_presence::{
    DiscordPresenceState,
    GamePresenceWatcher,
    PresenceBuildConfig,
    PresencePhase,
    PresenceStatusCallback,
};

use serde::{Deserialize, Serialize};
use std::error::Error;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
struct AppSettings {
    game_path: PathBuf,
    presence: PresenceBuildConfig,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            game_path: PathBuf::new(),
            presence: PresenceBuildConfig::default(),
        }
    }
}

fn valid_game_path(path: &Path) -> bool {
    path.join("game").join("citadel").is_dir()
}

fn load_settings(path: &Path) -> Result<AppSettings, Box<dyn Error>> {
    if !path.exists() {
        return Ok(AppSettings::default());
    }

    let contents = std::fs::read_to_string(path)?;
    let settings = serde_json::from_str(&contents)?;

    Ok(settings)
}

fn save_settings(
    path: &Path,
    settings: &AppSettings,
) -> Result<(), Box<dyn Error>> {
    let contents = serde_json::to_string_pretty(settings)?;
    std::fs::write(path, contents)?;

    Ok(())
}

fn configure_game_path(
    settings: &mut AppSettings,
    settings_file: &Path,
) -> Result<(), Box<dyn Error>> {
    if valid_game_path(&settings.game_path) {
        return Ok(());
    }

    println!();
    println!("Deadlock installation folder not configured.");
    println!("Enter the folder containing the game's 'game' directory.");
    println!();
    println!("Example:");
    println!(r"C:\Program Files (x86)\Steam\steamapps\common\Deadlock");
    println!();

    loop {
        print!("Deadlock folder: ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        let input = input.trim().trim_matches('"');

        if input.is_empty() {
            println!("Please enter a folder path.");
            continue;
        }

        let candidate = PathBuf::from(input);

        if !valid_game_path(&candidate) {
            println!("That folder doesn't contain game\\citadel.");
            println!("Please check the path and try again.");
            continue;
        }

        settings.game_path = candidate;
        save_settings(settings_file, settings)?;

        println!("Game installation path saved.");
        break;
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info")
    )
    .init();

    println!("======================================");
    println!("       Deadlock Discord Presence");
    println!("======================================");

    let executable = std::env::current_exe()?;
    let app_directory = executable
        .parent()
        .ok_or("Could not determine application directory")?;

    let settings_file = app_directory.join("settings.json");

    let mut settings = load_settings(&settings_file)?;

    configure_game_path(&mut settings, &settings_file)?;

    println!();
    println!("Deadlock folder: {:?}", settings.game_path);
    println!("Discord application: 1557070771946389555");
    println!();
    println!("Make sure Discord is running and logged in.");
    println!("Waiting for Deadlock...");
    println!("Press Ctrl+C to exit.");
    println!();

    let running = Arc::new(AtomicBool::new(true));

    let status_callback: PresenceStatusCallback =
        Arc::new(|phase| match phase {
            PresencePhase::Waiting => {
                log::info!("Status: Waiting for Deadlock");
            }
            PresencePhase::Connecting => {
                log::info!("Status: Connecting to Discord");
            }
            PresencePhase::Connected => {
                log::info!("Status: Discord Rich Presence connected");
            }
            PresencePhase::Error => {
                log::error!("Status: Discord Rich Presence error");
            }
        });

    let watcher = GamePresenceWatcher::new(
        settings.game_path.clone(),
        running.clone(),
        DiscordPresenceState::new(),
        Some(status_callback),
        settings.presence,
    );

    let mut watcher_task = tokio::spawn(async move {
        watcher.run().await;
    });

    tokio::select! {
        result = tokio::signal::ctrl_c() => {
            if let Err(error) = result {
                log::warn!("Could not listen for Ctrl+C: {error}");
            }

            log::info!("Shutting down...");
            running.store(false, Ordering::Relaxed);

            if let Err(error) = watcher_task.await {
                log::error!("Watcher task failed: {error}");
            }
        }

        result = &mut watcher_task => {
            if let Err(error) = result {
                log::error!("Watcher task failed: {error}");
            }
        }
    }

    println!();
    println!("Deadlock Discord Presence has stopped.");

    Ok(())
}
