#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod app_dirs;
mod file_picker;
mod image_fetch;
mod image_import;
mod steam;
mod steam_locate;

use app_dirs::PROJECT_DIRS;
use log::info;
use simple_logger::SimpleLogger;
use std::fs::create_dir_all;

fn main() {
    SimpleLogger::new()
        .with_level(log::LevelFilter::Info)
        .init()
        .unwrap();

    let cache_dir = PROJECT_DIRS.cache_dir();
    info!("Creating cache directory: {}", cache_dir.display());
    create_dir_all(cache_dir).unwrap();

    #[cfg(target_os = "linux")]
    let builder = tauri::Builder::<tauri::Cef>::default();
    #[cfg(not(target_os = "linux"))]
    let builder = tauri::Builder::<tauri::Wry>::default();

    builder
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            steam_locate::get_games,
            steam_locate::get_recent_steam_user,
            image_import::import_screenshots,
            file_picker::pick_screenshot_files,
            image_fetch::get_library_image
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
