use base64::{Engine as _, engine::general_purpose::STANDARD_NO_PAD};
use log::info;
use std::collections::HashMap;
use std::fs::read;
use std::path::{Path, PathBuf};
use steamy_vdf as vdf;
use walkdir::WalkDir;

const LIB_CACHE_PATH: &str = "appcache/librarycache/";

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
pub fn get_games() -> Result<Vec<(u32, String, String)>, String> {
    let steam_dir = steamlocate::locate().map_err(|_| "Failed to locate Steam installation")?;
    let apps_hash: HashMap<u32, steamlocate::App> = steam_dir
        .libraries()
        .map_err(|_| "Failed to get Steam libraries")?
        .filter_map(|library| {
            let library = library.ok()?;

            Some(
                library
                    .apps()
                    .filter_map(Result::ok)
                    .map(|app| (app.app_id, app))
                    .collect::<HashMap<_, _>>(),
            )
        })
        .flatten()
        .collect();
    let mut apps: Vec<u32> = apps_hash.keys().cloned().collect();
    apps.sort_unstable();
    let steam_path = steam_dir.path();

    let mut imgs: Vec<(u32, String, String)> = vec![];
    for appid in apps {
        let img = find_library_capsule(steam_path, appid)
            .and_then(|path| {
                info!("Found image path: {}", path.display());
                read(path).ok()
            })
            .unwrap_or_default();

        let b64_img = STANDARD_NO_PAD.encode(img);

        let app = apps_hash.get(&appid).unwrap();

        let name = app.name.as_ref().unwrap();

        imgs.push((appid, b64_img, name.to_string()));
    }

    Ok(imgs)
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
