use deadlock_discord_presence::{
    DiscordPresenceState, GamePresenceWatcher, PresenceBuildConfig, PresencePhase,
    PresenceStatusCallback,
};

use serde::{Deserialize, Serialize};
use std::error::Error;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

const DEADLOCK_APP_ID: &str = "1422450";

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
    Ok(serde_json::from_str(&contents)?)
}

fn save_settings(path: &Path, settings: &AppSettings) -> Result<(), Box<dyn Error>> {
    let contents = serde_json::to_string_pretty(settings)?;
    std::fs::write(path, contents)?;
    Ok(())
}

fn add_unique_path(paths: &mut Vec<PathBuf>, path: PathBuf) {
    let normalized = path.to_string_lossy().to_lowercase();

    if !paths
        .iter()
        .any(|existing| existing.to_string_lossy().to_lowercase() == normalized)
    {
        paths.push(path);
    }
}

#[cfg(windows)]
fn registry_path(key: &str, value_name: &str) -> Option<PathBuf> {
    let output = Command::new("reg")
        .args(["query", key, "/v", value_name])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    for line in stdout.lines() {
        let Some((name, value)) = line.trim().split_once("REG_SZ") else {
            continue;
        };

        if name.trim().eq_ignore_ascii_case(value_name) {
            let value = value.trim().trim_matches('"');

            if !value.is_empty() {
                return Some(PathBuf::from(value));
            }
        }
    }

    None
}

#[cfg(windows)]
fn steam_install_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();

    let registry_entries = [
        (r"HKCU\Software\Valve\Steam", "SteamPath"),
        (r"HKLM\SOFTWARE\WOW6432Node\Valve\Steam", "InstallPath"),
        (r"HKLM\SOFTWARE\Valve\Steam", "InstallPath"),
    ];

    for (key, value) in registry_entries {
        if let Some(path) = registry_path(key, value) {
            add_unique_path(&mut roots, path);
        }
    }

    if let Some(program_files) = std::env::var_os("ProgramFiles(x86)") {
        add_unique_path(&mut roots, PathBuf::from(program_files).join("Steam"));
    }

    if let Some(program_files) = std::env::var_os("ProgramFiles") {
        add_unique_path(&mut roots, PathBuf::from(program_files).join("Steam"));
    }

    roots
}

#[cfg(not(windows))]
fn steam_install_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();

    if let Some(home) = std::env::var_os("HOME") {
        let home = PathBuf::from(home);

        add_unique_path(&mut roots, home.join(".steam").join("steam"));
        add_unique_path(&mut roots, home.join(".local").join("share").join("Steam"));
    }

    roots
}

fn steam_library_paths() -> Vec<PathBuf> {
    let roots = steam_install_roots();
    let mut libraries = Vec::new();

    for root in &roots {
        add_unique_path(&mut libraries, root.clone());

        let vdf_path = root.join("steamapps").join("libraryfolders.vdf");

        let Ok(contents) = std::fs::read_to_string(vdf_path) else {
            continue;
        };

        let Ok(pattern) = regex::Regex::new(r#""path"\s*"([^"]+)""#) else {
            continue;
        };

        for capture in pattern.captures_iter(&contents) {
            if let Some(path) = capture.get(1) {
                let decoded = path.as_str().replace("\\\\", "\\");
                add_unique_path(&mut libraries, PathBuf::from(decoded));
            }
        }
    }

    libraries
}

fn find_deadlock_in_library(library: &Path) -> Option<PathBuf> {
    let common = library.join("steamapps").join("common");
    let direct = common.join("Deadlock");

    if valid_game_path(&direct) {
        return Some(direct);
    }

    let entries = std::fs::read_dir(&common).ok()?;

    for entry in entries.flatten() {
        let candidate = entry.path();

        if entry
            .file_name()
            .to_string_lossy()
            .eq_ignore_ascii_case("Deadlock")
            && valid_game_path(&candidate)
        {
            return Some(candidate);
        }
    }

    None
}

fn detect_deadlock_installation() -> Option<PathBuf> {
    for library in steam_library_paths() {
        if let Some(game_path) = find_deadlock_in_library(&library) {
            return Some(game_path);
        }
    }

    None
}

fn find_steam_executable() -> Option<PathBuf> {
    for root in steam_install_roots() {
        let executable = root.join("steam.exe");

        if executable.is_file() {
            return Some(executable);
        }
    }

    None
}

fn deadlock_is_running() -> bool {
    let mut system = sysinfo::System::new();

    system.refresh_processes_specifics(
        sysinfo::ProcessesToUpdate::All,
        true,
        sysinfo::ProcessRefreshKind::nothing(),
    );

    let process_names = ["project8.exe", "deadlock.exe", "project8"];

    process_names.iter().any(|name| {
        system
            .processes_by_exact_name(OsStr::new(name))
            .next()
            .is_some()
    })
}

fn launch_deadlock() -> Result<(), Box<dyn Error>> {
    if deadlock_is_running() {
        log::info!("Deadlock is already running. Connecting to existing session.");
        return Ok(());
    }

    log::info!("Launching Deadlock through Steam...");

    #[cfg(windows)]
    {
        if let Some(steam_exe) = find_steam_executable() {
            Command::new(steam_exe)
                .arg("-applaunch")
                .arg(DEADLOCK_APP_ID)
                .spawn()?;
        } else {
            Command::new("cmd")
                .args(["/C", "start", "", "steam://rungameid/1422450"])
                .spawn()?;
        }
    }

    #[cfg(not(windows))]
    {
        Command::new("steam")
            .arg("-applaunch")
            .arg(DEADLOCK_APP_ID)
            .spawn()?;
    }

    log::info!("Deadlock launch request sent to Steam.");

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    println!("======================================");
    println!("       Deadlock Discord Presence");
    println!("======================================");

    let executable = std::env::current_exe()?;
    let app_directory = executable
        .parent()
        .ok_or("Could not determine application directory")?;

    let settings_file = app_directory.join("settings.json");
    let mut settings = load_settings(&settings_file)?;

    if !valid_game_path(&settings.game_path) {
        println!();
        println!("Detecting Deadlock installation through Steam...");

        let detected_path = detect_deadlock_installation()
            .ok_or(
                "Could not automatically find Deadlock. Make sure Steam is installed and Deadlock is installed in one of your Steam libraries."
            )?;

        settings.game_path = detected_path;
        save_settings(&settings_file, &settings)?;

        println!("Deadlock installation detected and saved.");
    } else {
        println!("Using saved Deadlock installation.");
    }

    println!();
    println!("Deadlock folder: {:?}", settings.game_path);
    println!("Discord application: 1557070771946389555");
    println!();

    println!("Make sure Discord is running and logged in.");

    launch_deadlock()?;

    println!("Waiting for Deadlock...");
    println!("Press Ctrl+C to exit.");
    println!();

    let running = Arc::new(AtomicBool::new(true));

    let status_callback: PresenceStatusCallback = Arc::new(|phase| match phase {
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
