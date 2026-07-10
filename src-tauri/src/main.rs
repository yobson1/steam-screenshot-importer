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

#[cfg(target_os = "linux")]
type AppRuntime = tauri::Cef;
#[cfg(not(target_os = "linux"))]
type AppRuntime = tauri::Wry;

use app_dirs::PROJECT_DIRS;
use log::info;
use simple_logger::SimpleLogger;
#[cfg(debug_assertions)]
use specta_typescript::Typescript;
use std::fs::create_dir_all;
use tauri_specta::{Builder, ErrorHandlingMode, collect_commands};

#[cfg(debug_assertions)]
const TYPESCRIPT_BINDINGS_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../src/bindings.ts");

fn main() {
    SimpleLogger::new()
        .with_level(log::LevelFilter::Info)
        .init()
        .unwrap();

    let cache_dir = PROJECT_DIRS.cache_dir();
    info!("Creating cache directory: {}", cache_dir.display());
    create_dir_all(cache_dir).unwrap();

    let command_builder = Builder::<AppRuntime>::new()
        .commands(collect_commands![
            steam_locate::get_games,
            steam_locate::get_recent_steam_user,
            image_import::import_screenshots,
            file_picker::pick_screenshot_files,
        ])
        .error_handling(ErrorHandlingMode::Throw);

    #[cfg(debug_assertions)]
    command_builder
        .export(Typescript::default(), TYPESCRIPT_BINDINGS_PATH)
        .expect("failed to export TypeScript bindings");

    let builder = tauri::Builder::<AppRuntime>::default();

    builder
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(command_builder.invoke_handler())
        .setup(move |app| {
            command_builder.mount_events(app);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
