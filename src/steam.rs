use log::info;
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};
use steamworks::Client;
use steamworks::sys::SteamAPI_IsSteamRunning as is_steam_running;

pub fn open_steam_section(section: &str) -> Result<(), String> {
    let open_command = if cfg!(target_os = "windows") {
        "explorer"
    } else if cfg!(target_os = "linux") {
        "xdg-open"
    } else {
        return Err("Unsupported OS".to_string());
    };

    let status = Command::new(open_command)
        .arg(format!("steam://open/{section}"))
        .spawn()
        .map_err(|error| format!("Failed to open Steam: {error}"))?
        .wait()
        .map_err(|error| format!("Failed while waiting for Steam to open: {error}"))?;

    if cfg!(target_os = "linux") && !status.success() {
        return Err(format!("Failed to open Steam: {status}"));
    }

    Ok(())
}

pub fn initialize_steam(app_id: u32) -> Result<Client, String> {
    if unsafe { is_steam_running() } {
        Client::init_app(app_id).map_err(|_| "Failed to initialize steamworks!\nMake sure steam is open and you own the game you're attempting to import for.".to_string())
    } else {
        open_steam_section("main")?;
        wait_for_steam(app_id)
    }
}

fn wait_for_steam(app_id: u32) -> Result<Client, String> {
    let start = Instant::now();
    while start.elapsed().as_secs() < 20 {
        if let Ok(client) = Client::init_app(app_id) {
            info!("Steam started successfully");
            return Ok(client);
        }
        thread::sleep(Duration::from_millis(500));
    }

    Err("Steam is not running and failed to automatically start".to_string())
}
