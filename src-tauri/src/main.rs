#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use directories::ProjectDirs;
use image::imageops::{resize, FilterType};
use image::io::Reader as ImageReader;
use image::ImageOutputFormat;
use log::{info, warn};
use simple_logger::SimpleLogger;
use std::collections::HashMap;
use std::ffi::CString;
use std::fs::{create_dir_all, read, remove_dir_all, File};
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::process::Command;
use steamlocate::{SteamApp, SteamDir};
use steamworks::sys::SteamAPI_ISteamScreenshots_AddScreenshotToLibrary as add_screenshot_to_library;
use steamworks::sys::SteamAPI_SteamScreenshots_v003 as get_steam_screenshots;
use steamworks::Client;
use steamy_vdf as vdf;

const LIB_CACHE_PATH: &str = "appcache\\librarycache\\";

#[tauri::command]
fn get_games() -> Vec<(u32, String, String)> {
    let mut steamdir: SteamDir = SteamDir::locate().unwrap();
    let apps_hash: HashMap<u32, Option<SteamApp>> = steamdir.apps().clone();
    let apps: Vec<u32> = apps_hash.keys().cloned().collect();
    let steam_path: PathBuf = steamdir.path;

    let mut imgs: Vec<(u32, String, String)> = vec![];
    for appid in apps {
        let img_path: PathBuf = steam_path
            .join(LIB_CACHE_PATH)
            .join(format!("{}_library_600x900.jpg", appid));
        let img: Vec<u8> = match read(&img_path) {
            Ok(img) => img,
            Err(_) => vec![],
        };
        let b64_img: String = base64::encode(img);
        let app = apps_hash.get(&appid).unwrap().as_ref().unwrap();
        let name = app.name.as_ref().unwrap();
        imgs.push((appid, b64_img, name.to_string()));
    }

    return imgs;
}

#[tauri::command]
fn get_recent_steam_user() -> String {
    let steamdir: SteamDir = SteamDir::locate().unwrap();
    let steam_path: PathBuf = steamdir.path;
    let vdf_path: PathBuf = steam_path.join("config").join("loginusers.vdf");

    let loginusers = vdf::load(&vdf_path).unwrap();
    let users: Vec<vdf::Entry> = loginusers
        .lookup("users")
        .unwrap()
        .as_table()
        .unwrap()
        .values()
        .cloned()
        .collect();

    let mut steam_user: &str = "";
    for i in 0..users.len() {
        let user = &users[i];
        let recent_entry = match user.lookup("MostRecent") {
            Some(entry) => entry,
            None => continue,
        };
        let is_most_recent = recent_entry.to::<bool>().unwrap();
        if is_most_recent {
            steam_user = user.lookup("PersonaName").unwrap().as_str().unwrap();
        }
    }

    return steam_user.to_string();
}

const PREVIEW_WIDTH: u32 = steamworks::sys::k_ScreenshotThumbWidth as u32;

#[tauri::command]
fn import_screenshots(file_paths: Vec<String>, app_id: u32) {
    // TODO: Spin up a seperate thread for this
    // and then use events to communicate back to the UI
    // to update the progress
    info!(
        "Uploading {} screenshots under AppID {}",
        file_paths.len(),
        app_id
    );

    let num_of_files = file_paths.len();
    if num_of_files > 0 {
        let project_dirs = ProjectDirs::from("com", "yob", "ssi").unwrap();
        let cache_dir = project_dirs.cache_dir();
        create_dir_all(&cache_dir).unwrap();

        // Initialize steamworks - we don't actually need to use the client object since
        // it doesn't formally implement the ISteamScreenshots interface and we're using
        // the raw bindings for that later.
        // TODO: Handle errors, usually will be from steam not being open InitFailed error
        // Also need to test what happens if attempted on an app_id the user doesn't own
        let (_client, single) = Client::init_app(app_id).unwrap();

        for file_path in file_paths {
            let img_path = Path::new(&file_path);
            let img_name = img_path.file_stem().unwrap().to_str().unwrap();
            let extension = img_path.extension().unwrap().to_str().unwrap();

            let new_file_name = format!("{}_{}.jpg", img_name, app_id);
            let new_thumbnail_name = format!("{}_{}_thumb.jpg", img_name, app_id);

            info!("New file name: {}", new_file_name);

            // Load original image
            info!("Loading image: {}", img_path.display());
            let img = ImageReader::open(file_path.as_str())
                .unwrap()
                .decode()
                .unwrap();

            // Convert to jpg if needed
            // TODO: Check these
            // steamMaxSideSize = 16000;
            // steamMaxResolution = 26210175;
            let new_img_path = cache_dir.join(&new_file_name);

            if extension != "jpg" && extension != "jpeg" {
                info!("Converting image {}.{} to jpg", img_name, extension);
                let file = File::create(&new_img_path).unwrap();
                let mut writer = BufWriter::new(file);
                img.write_to(&mut writer, ImageOutputFormat::Jpeg(95))
                    .unwrap(); // TODO: Make the quality configurable
            } else {
                info!("Copying image {}.{}", img_name, extension);
                img.save(&new_img_path).unwrap();
            }

            // Create thumbnail image
            info!("Resizing image {}.{} for thumbnail", img_name, extension);

            let preview_img_path = cache_dir.join(&new_thumbnail_name);

            let preview_height = (PREVIEW_WIDTH * img.height()) / img.width();
            let preview_img = resize(&img, PREVIEW_WIDTH, preview_height, FilterType::Lanczos3);
            let file = File::create(&preview_img_path).unwrap();
            let mut writer = BufWriter::new(&file);
            preview_img
                .write_to(&mut writer, ImageOutputFormat::Jpeg(95))
                .unwrap();

            // Import screenshot
            info!(
                "Importing screenshot {} {}",
                new_img_path.display(),
                preview_img_path.display()
            );
            unsafe {
                let screenshots = get_steam_screenshots();
                let screenshot_path = CString::new(&*new_img_path.to_string_lossy()).unwrap();
                let thumbnail_path = CString::new(&*preview_img_path.to_string_lossy()).unwrap();
                add_screenshot_to_library(
                    screenshots,
                    screenshot_path.as_ptr(),
                    thumbnail_path.as_ptr(),
                    img.width().try_into().unwrap(),
                    img.height().try_into().unwrap(),
                );
                single.run_callbacks();
                info!("Import of {}.{} complete", img_name, extension);
            }
        }

        info!(
            "Import of {} images complete, opening steam screenshots window",
            num_of_files
        );

        // Open the steam screenshots window for upload
        Command::new("explorer")
            .arg(format!("steam://open/screenshots/{}", app_id))
            .spawn()
            .unwrap();

        // Empty the cache
        remove_dir_all(&cache_dir).unwrap();
    } else {
        warn!("Got no screenshots to import");
    }
}

fn main() {
    SimpleLogger::new()
        .with_level(log::LevelFilter::Info)
        .init()
        .unwrap();

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_games,
            get_recent_steam_user,
            import_screenshots
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
