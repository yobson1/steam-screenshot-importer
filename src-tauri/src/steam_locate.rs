use crate::image_fetch;
use base64::{Engine as _, engine::general_purpose::STANDARD_NO_PAD};
use log::info;
use serde::Serialize;
use std::fs::read;
use std::path::{Path, PathBuf};
use steamy_vdf as vdf;
use walkdir::WalkDir;

const LIB_CACHE_PATH: &str = "appcache/librarycache/";

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Game {
    app_id: u32,
    image_src: String,
    app_name: String,
}

struct LocalGame {
    app_id: u32,
    image_src: Option<String>,
    app_name: String,
}

fn find_library_capsule(steam_path: &Path, appid: u32) -> Option<PathBuf> {
    let app_cache_path = steam_path.join(LIB_CACHE_PATH).join(appid.to_string());

    for entry in WalkDir::new(app_cache_path)
        .into_iter()
        .filter_map(Result::ok)
    {
        let file_name = entry.file_name().to_string_lossy();

        if file_name == "library_capsule.jpg" || file_name == "library_600x900.jpg" {
            return Some(entry.path().to_path_buf());
        }
    }

    None
}

#[tauri::command]
pub async fn get_games() -> Result<Vec<Game>, String> {
    let games = get_local_games()?;
    let missing_app_ids: Vec<u32> = games
        .iter()
        .filter(|game| game.image_src.is_none())
        .map(|game| game.app_id)
        .collect();
    let fetched_images = image_fetch::get_library_images(&missing_app_ids).await;

    Ok(games
        .into_iter()
        .map(|game| {
            let image_src = game
                .image_src
                .or_else(|| fetched_images.get(&game.app_id).cloned())
                .unwrap_or_default();

            Game {
                app_id: game.app_id,
                image_src,
                app_name: game.app_name,
            }
        })
        .collect())
}

fn get_local_games() -> Result<Vec<LocalGame>, String> {
    let steam_dir = steamlocate::locate().map_err(|_| "Failed to locate Steam installation")?;
    let libraries = steam_dir
        .libraries()
        .map_err(|_| "Failed to get Steam libraries")?;
    let mut apps = Vec::new();

    for library in libraries.filter_map(Result::ok) {
        apps.extend(library.apps().filter_map(Result::ok));
    }

    apps.sort_unstable_by_key(|app| app.app_id);
    apps.dedup_by_key(|app| app.app_id);

    let steam_path = steam_dir.path();
    let mut games = vec![];

    for app in apps {
        let Some(app_name) = app.name else {
            continue;
        };

        let image_src = find_library_capsule(steam_path, app.app_id)
            .and_then(|path| {
                info!("Found image path: {}", path.display());
                read(path).ok()
            })
            .map(|img| format!("data:image/jpeg;base64,{}", STANDARD_NO_PAD.encode(img)));

        games.push(LocalGame {
            app_id: app.app_id,
            image_src,
            app_name,
        });
    }

    Ok(games)
}

#[tauri::command]
pub fn get_recent_steam_user() -> Result<String, String> {
    let steam_dir = steamlocate::locate().map_err(|_| "Failed to locate Steam installation")?;
    let steam_path = steam_dir.path();
    let vdf_path: PathBuf = steam_path.join("config").join("loginusers.vdf");

    let loginusers = vdf::load(&vdf_path).unwrap();
    let users: Vec<vdf::Entry> = loginusers
        .lookup("users")
        .ok_or("Failed to get local Steam users")?
        .as_table()
        .ok_or("Failed to convert local Steam users to table")?
        .values()
        .cloned()
        .collect();

    let mut steam_user: &str = "";
    for user in &users {
        if let Some(recent_entry) = user.lookup("AutoLogin")
            && recent_entry.to::<bool>().unwrap_or(false)
        {
            steam_user = user
                .lookup("PersonaName")
                .ok_or("Failed to get Steam username")?
                .as_str()
                .ok_or("Failed to convert Steam username to string")?;
            break;
        }
    }

    Ok(steam_user.to_string())
}
