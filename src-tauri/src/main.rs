#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use chrono::Local;
use directories::ProjectDirs;
use image::imageops::{resize, FilterType};
use image::io::Reader as ImageReader;
use image::ImageOutputFormat;
use log::{error, info};
use serde::{Deserialize, Serialize};
use simple_logger::SimpleLogger;
use std::collections::HashMap;
use std::fs::{create_dir_all, read, File};
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::process::Command;
use steamlocate::{SteamApp, SteamDir};
use steamworks::{Client, FileType};
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

const PREVIEW_WIDTH: u32 = 200;

#[derive(Serialize, Deserialize, Debug)]
struct ScreenshotVDFEntry {
    #[serde(rename = "type")]
    screen_type: u8,
    filename: String,
    thumbnail: String,
    vrfilename: String,
    imported: bool,
    width: u32,
    height: u32,
    gameid: u32,
    creation: i64,
    caption: String,
    #[serde(rename = "Permissions")]
    permissions: String,
    hscreenshot: String,
}

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

    // Look through file_paths
    if file_paths.len() > 0 {
        let project_dirs = ProjectDirs::from("com", "yob", "ssi").unwrap();
        let cache_dir = project_dirs.cache_dir();
        create_dir_all(&cache_dir).unwrap();

        // Initialize steamworks
        // TODO: Handle errors, usually will be from steam not being open InitFailed error
        let (client, single) = Client::init_app(app_id).unwrap();
        let user_id = client.user().steam_id().account_id();

        info!("Steam user ID: {}", user_id.raw());

        // Get the app's screenshot directory
        // Using steamlocate, the user ID & app ID
        let steamdir: SteamDir = SteamDir::locate().unwrap();
        let steam_path: PathBuf = steamdir.path;
        let screenshots_root: PathBuf = steam_path
            .join("userdata")
            .join(format!("{}", user_id.raw()));
        let screenshots_path = screenshots_root
            .join("760")
            .join("remote")
            .join(&app_id.to_string())
            .join("screenshots");
        let thumbnails_path = screenshots_path.join("thumbnails");
        let vdf_path = screenshots_root.join("screenshots.vdf");

        info!("Screenshots path: {}", screenshots_path.display());
        info!("Thumbnails path: {}", thumbnails_path.display());

        create_dir_all(&thumbnails_path).unwrap();

        // Get the name the image will be saved in using the current date & time
        let date = Local::now();
        let new_file_name = date.format("%Y%m%d%H%M%S_");

        let mut i = 0;
        for file_path in file_paths {
            i += 1;
            let new_file_name = new_file_name.to_string() + &i.to_string() + ".jpg";

            info!("New file name: {}", new_file_name);

            let client = client.clone();
            let ugc = client.ugc();
            let utils = client.utils();

            // Create downscaled preview image
            let img_path = Path::new(&file_path);
            let img_name = img_path.file_stem().unwrap().to_str().unwrap();
            let extension = img_path.extension().unwrap().to_str().unwrap();

            // Load original image
            info!("Loading image: {}", img_path.display());
            let img = ImageReader::open(file_path.as_str())
                .unwrap()
                .decode()
                .unwrap();

            // Create preview image
            info!("Resizing image {}.{} for thumbnail", img_name, extension);

            let preview_height = (PREVIEW_WIDTH * img.height()) / img.width();
            // let preview_img = img.thumbnail(PREVIEW_WIDTH, preview_height);
            let preview_img = resize(&img, PREVIEW_WIDTH, preview_height, FilterType::Lanczos3);
            let preview_img_path = thumbnails_path.join(&new_file_name);
            let file = File::create(&preview_img_path).unwrap();
            let mut writer = BufWriter::new(&file);
            preview_img
                .write_to(&mut writer, ImageOutputFormat::Jpeg(95))
                .unwrap();

            // Convert to jpg if needed
            // TODO: Check these
            // steamMaxSideSize = 16000;
            // steamMaxResolution = 26210175;
            let new_img_path = screenshots_path.join(&new_file_name);

            if extension != "jpg" && extension != "jpeg" {
                info!("Converting image {}.{} to jpg", img_name, extension);
                let file = File::create(&new_img_path).unwrap();
                let mut writer = BufWriter::new(file);
                img.write_to(&mut writer, ImageOutputFormat::Jpeg(95))
                    .unwrap(); // TODO: Make the quality configurable
            } else {
                info!("Copying image {}.{}", img_name, extension);
                let file = File::create(&new_img_path).unwrap();
                let mut writer = BufWriter::new(file);
                img.write_to(&mut writer, ImageOutputFormat::Jpeg(95))
                    .unwrap();
            }

            // Prepare VDF data
            let entry = ScreenshotVDFEntry {
                screen_type: 1,
                filename: app_id.to_string() + "/screenshots/" + &new_file_name,
                thumbnail: app_id.to_string() + "/screenshots/thumbnails" + &new_file_name,
                vrfilename: "".to_string(),
                imported: false,
                width: img.width(),
                height: img.height(),
                gameid: app_id,
                creation: date.timestamp(),
                caption: "".to_string(),
                permissions: "".to_string(),
                hscreenshot: "".to_string(),
            };
            let vdf_string = vdf_serde::to_string(&entry).unwrap();

            info!("VDF Data:\n{}", vdf_string);

            // Upload screenshot
            let app_id = utils.app_id();

            ugc.create_item(app_id, FileType::Screenshot, move |result| {
                let ugc = client.ugc();

                let (file_id, _bool) = match result {
                    Ok(item) => item,
                    Err(err) => {
                        error!("{}", err);
                        return;
                    }
                };

                info!("Created UGC with ID {}", file_id.0);

                info!(
                    "Uploading image {} {}",
                    new_img_path.to_str().unwrap(),
                    preview_img_path.to_str().unwrap()
                );

                // FIXME: Hangs after this
                // Might be better to just use `explorer steam://open/screenshots/<appid>` with std::process::Command
                let update_watch = ugc
                    .start_item_update(app_id, file_id)
                    // .title("Screenshot")
                    // .description("Image imported using yobson's SSI tool")
                    // .preview_path(&preview_img_path)
                    .content_path(&new_img_path)
                    .submit(None, |result| {
                        info!("Submit callback");

                        let (file_id, _bool) = match result {
                            Ok(item) => item,
                            Err(err) => {
                                error!("{}", err);
                                return;
                            }
                        };

                        info!("Uploaded image {}", file_id.0);
                    });

                let (status, progress, total) = update_watch.progress();
                info!(
                    "{:#?} {}%({}/{})",
                    status,
                    progress / total,
                    progress,
                    total
                );
            });
        }

        loop {
            info!("Running callbacks");
            single.run_callbacks();
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
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
