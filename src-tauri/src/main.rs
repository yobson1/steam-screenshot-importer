use atomic_float::AtomicF32;
use directories::ProjectDirs;
use image::imageops::{resize, FilterType};
use image::io::Reader as ImageReader;
use image::ImageOutputFormat;
use lazy_static::lazy_static;
use log::{error, info, warn};
use rayon::prelude::*;
use simple_logger::SimpleLogger;
use std::collections::HashMap;
use std::ffi::CString;
use std::fs::{create_dir, create_dir_all, read, remove_dir_all, File};
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{atomic::Ordering, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use steamlocate::{SteamApp, SteamDir};
use steamworks::sys::EHTTPMethod;
use steamworks::sys::SteamAPICall_t;
use steamworks::sys::SteamAPI_ISteamHTTP_CreateHTTPRequest as steam_http_create;
use steamworks::sys::SteamAPI_ISteamHTTP_GetHTTPDownloadProgressPct as http_get_download_progress;
use steamworks::sys::SteamAPI_ISteamHTTP_GetHTTPResponseBodyData as http_get_response_body_data;
use steamworks::sys::SteamAPI_ISteamHTTP_GetHTTPResponseBodySize as http_get_response_body_size;
use steamworks::sys::SteamAPI_ISteamHTTP_ReleaseHTTPRequest as steam_http_release;
use steamworks::sys::SteamAPI_ISteamHTTP_SendHTTPRequest as steam_http_send;
use steamworks::sys::SteamAPI_ISteamScreenshots_AddScreenshotToLibrary as add_screenshot_to_library;
use steamworks::sys::SteamAPI_IsSteamRunning as is_steam_running;
use steamworks::sys::SteamAPI_SteamHTTP_v003 as get_steam_http;
use steamworks::sys::SteamAPI_SteamScreenshots_v003 as get_steam_screenshots;
use steamworks::{Client, SingleClient};
use steamy_vdf as vdf;

const LIB_CACHE_PATH: &str = "appcache\\librarycache\\";
const PROGRESS_EVENT: &str = "screenshotImportProgress";
const ERROR_EVENT: &str = "screenshotImportError";

lazy_static! {
    static ref PROJECT_DIRS: ProjectDirs = ProjectDirs::from("com", "yob", "ssi").unwrap();
}

unsafe fn steam_http_get(url: &str) -> String {
    let url = CString::new(url).unwrap();
    let http = get_steam_http();
    let request_handle = steam_http_create(http, EHTTPMethod::k_EHTTPMethodGET, url.as_ptr());
    let mut api_call: SteamAPICall_t = 0;
    steam_http_send(http, request_handle, &mut api_call as *mut _);

    let mut progress: f32 = 0.0;
    loop {
        http_get_download_progress(http, request_handle, &mut progress as *mut _);

        if progress == 100.0 {
            break;
        }
    }

    let mut buf_size: u32 = 0;

    http_get_response_body_size(http, request_handle, &mut buf_size as *mut _);

    let mut buf: Vec<u8> = vec![0; buf_size.try_into().unwrap()];
    http_get_response_body_data(http, request_handle, &mut buf[0] as *mut _, buf_size);

    steam_http_release(http, request_handle);

    String::from_utf8(buf).unwrap()
}

fn open_steam_section(section: &str) {
    let open_command = if cfg!(target_os = "windows") {
        "explorer"
    } else if cfg!(target_os = "linux") {
        "xdg-open"
    } else {
        panic!("Unsupported OS");
    };

    Command::new(open_command)
        .arg(format!("steam://open/{}", section))
        .spawn()
        .unwrap();
}

#[tauri::command]
fn get_games() -> Result<Vec<(u32, String, String)>, String> {
    let mut steamdir: SteamDir = SteamDir::locate().ok_or("Failed to locate Steam installation")?;
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

    Ok(imgs)
}

#[tauri::command]
fn get_recent_steam_user() -> Result<String, String> {
    let steamdir: SteamDir = SteamDir::locate().ok_or("Failed to locate Steam installation")?;
    let steam_path: PathBuf = steamdir.path;
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
    for i in 0..users.len() {
        let user = &users[i];
        if let Some(recent_entry) = user.lookup("MostRecent") {
            if recent_entry.to::<bool>().unwrap_or(false) {
                steam_user = user
                    .lookup("PersonaName")
                    .ok_or("Failed to get Steam username")?
                    .as_str()
                    .ok_or("Failed to convert Steam username to string")?;
                break;
            }
        }
    }

    Ok(steam_user.to_string())
}

const THUMB_WIDTH: u32 = steamworks::sys::k_ScreenshotThumbWidth as u32;
const MAX_SIDE: u32 = 16000;
const MAX_RESOLUTION: u32 = 26210175;

#[tauri::command]
async fn import_screenshots(file_paths: Vec<String>, app_id: u32, window: tauri::Window) -> String {
    info!(
        "Importing {} screenshots under AppID {}",
        file_paths.len(),
        app_id
    );

    let num_of_files = file_paths.len();
    if num_of_files == 0 {
        warn!("Got no screenshots to import");
        return "No screenshots to import!".to_string();
    }

    let cache_dir = PROJECT_DIRS.cache_dir();

    // Check if steam is running and initialize client
    let (client, single) = initialize_steam(app_id).unwrap();

    // window.emit(PROGRESS_EVENT, "Import started").unwrap();

    // Wrap shared resources in Arc for thread-safe sharing
    let window = Arc::new(window);
    let cache_dir = Arc::new(cache_dir);
    let single = Arc::new(Mutex::new(single));

    // Progress bar
    let screenshots_completed = AtomicF32::new(0.0);

    // Process screenshots in parallel
    file_paths.par_iter().for_each(|file_path| {
        let window = Arc::clone(&window);
        let cache_dir = Arc::clone(&cache_dir);
        let single = Arc::clone(&single);

        import_single_screenshot(
            file_path,
            app_id,
            &window,
            &cache_dir,
            &single,
            &screenshots_completed,
            num_of_files,
        );
    });

    drop(client);

    info!(
        "Import of {} images complete, opening steam screenshots window",
        num_of_files
    );

    // Open the steam screenshots window for upload
    open_steam_section(&format!("screenshots/{}", app_id));

    // Empty the cache
    info!("Emptying cache");
    remove_dir_all(*cache_dir)
        .and_then(|_| create_dir(*cache_dir))
        .unwrap();

    String::default()
}

fn initialize_steam(app_id: u32) -> Result<(Client, SingleClient), String> {
    if unsafe { is_steam_running() } {
        Client::init_app(app_id).map_err(|_| "Failed to initialize steamworks!\nMake sure steam is open and you own the game you're attempting to import for.".to_string())
    } else {
        open_steam_section("main");
        wait_for_steam(app_id)
    }
}

fn wait_for_steam(app_id: u32) -> Result<(Client, SingleClient), String> {
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

fn import_single_screenshot(
    file_path: &str,
    app_id: u32,
    window: &tauri::Window,
    cache_dir: &Path,
    single: &Mutex<steamworks::SingleClient>,
    screenshots_completed: &AtomicF32,
    total_screenshots: usize,
) {
    let img_path = Path::new(file_path);
    let img_name = img_path.file_stem().unwrap().to_str().unwrap();
    let extension = img_path.extension().unwrap().to_str().unwrap();

    // window
    //     .emit(
    //         PROGRESS_EVENT,
    //         &format!("Loading {}.{}", img_name, extension),
    //     )
    //     .unwrap();

    let new_file_name = format!("{}_{}.jpg", img_name, app_id);
    let new_thumbnail_name = format!("{}_{}_thumb.jpg", img_name, app_id);

    info!("New file name: {}", new_file_name);

    // Load original image
    info!("Loading image: {}", img_path.display());
    let mut img = match ImageReader::open(file_path).unwrap().decode() {
        Ok(img) => img,
        Err(e) => {
            window
                .emit(ERROR_EVENT, &format!("{}.{}\n{}", img_name, extension, e))
                .unwrap();
            error!("{}", e);
            thread::sleep(Duration::from_millis(2500));
            return;
        }
    };

    // Convert to jpg or downscale if needed
    let new_img_path = cache_dir.join(&new_file_name);

    if img.width() > MAX_SIDE
        || img.height() > MAX_SIDE
        || img.width() * img.height() > MAX_RESOLUTION
    {
        warn!(
            "Image {}.{} is too large to be imported, it will be downscaled",
            img_name, extension
        );

        // window
        //     .emit(
        //         PROGRESS_EVENT,
        //         &format!(
        //             "Resizing {}.{} to fit within steam's limits",
        //             img_name, extension
        //         ),
        //     )
        //     .unwrap();

        let scale_factor = f32::min(
            MAX_SIDE as f32 / f32::max(img.width() as f32, img.height() as f32),
            MAX_RESOLUTION as f32 / (img.width() * img.height()) as f32,
        );
        let new_width = (img.width() as f32 * scale_factor) as u32;
        let new_height = (img.height() as f32 * scale_factor) as u32;

        if new_width <= 0 || new_height <= 0 {
            warn!(
                "Image {}.{} is too large to be imported and cannot be downscaled correctly, it will be skipped",
                img_name, extension
            );

            // window
            //     .emit(
            //         PROGRESS_EVENT,
            //         &format!(
            //             "Skipping {}.{} as it is too large to be imported",
            //             img_name, extension
            //         ),
            //     )
            //     .unwrap();

            return;
        }

        img = img.resize_exact(new_width, new_height, FilterType::Lanczos3);

        info!(
            "{}.{} new size: {}x{}",
            img_name, extension, new_width, new_height
        );
    }

    if extension != "jpg" && extension != "jpeg" {
        info!("Converting image {}.{} to jpg", img_name, extension);
        // window
        //     .emit(
        //         PROGRESS_EVENT,
        //         &format!("Converting image {}.{} to jpeg", img_name, extension),
        //     )
        //     .unwrap();
        let file = File::create(&new_img_path).unwrap();
        let mut writer = BufWriter::new(file);
        img.write_to(&mut writer, ImageOutputFormat::Jpeg(95))
            .unwrap(); // TODO: Make the quality configurable
    } else {
        info!("Copying image {}.{}", img_name, extension);
        img.save(&new_img_path).unwrap();
    }

    update_progress(window, screenshots_completed, total_screenshots, 0.3);

    // Create thumbnail image
    info!("Resizing image {}.{} for thumbnail", img_name, extension);
    // window
    //     .emit(
    //         PROGRESS_EVENT,
    //         &format!("Resizing image {}.{} for thumbnail", img_name, extension),
    //     )
    //     .unwrap();

    let thumb_img_path = cache_dir.join(&new_thumbnail_name);

    let thumb_height = (THUMB_WIDTH * img.height()) / img.width();
    let thumb_img = resize(&img, THUMB_WIDTH, thumb_height, FilterType::Lanczos3);
    let file = File::create(&thumb_img_path).unwrap();
    let mut writer = BufWriter::new(&file);
    thumb_img
        .write_to(&mut writer, ImageOutputFormat::Jpeg(95))
        .unwrap();

    update_progress(window, screenshots_completed, total_screenshots, 0.4);

    // Import screenshot
    info!(
        "Importing screenshot {} {}",
        new_img_path.display(),
        thumb_img_path.display()
    );
    unsafe {
        let screenshots = get_steam_screenshots();
        let screenshot_path = CString::new(&*new_img_path.to_string_lossy()).unwrap();
        let thumbnail_path = CString::new(&*thumb_img_path.to_string_lossy()).unwrap();
        add_screenshot_to_library(
            screenshots,
            screenshot_path.as_ptr(),
            thumbnail_path.as_ptr(),
            img.width().try_into().unwrap(),
            img.height().try_into().unwrap(),
        );
        single.lock().unwrap().run_callbacks();
    }
    info!("Import of {}.{} complete", img_name, extension);

    update_progress(window, screenshots_completed, total_screenshots, 0.3);
}

fn update_progress(
    window: &tauri::Window,
    screenshots_completed: &AtomicF32,
    total_screenshots: usize,
    step_progress: f32,
) {
    let completed = screenshots_completed.fetch_add(step_progress, Ordering::SeqCst);
    let progress = ((completed + step_progress) / total_screenshots as f32) * 100.0;
    window.emit(PROGRESS_EVENT, progress).unwrap();
}

fn main() {
    SimpleLogger::new()
        .with_level(log::LevelFilter::Info)
        .init()
        .unwrap();

    let cache_dir = PROJECT_DIRS.cache_dir();
    info!("Creating cache directory: {}", cache_dir.display());
    create_dir_all(&cache_dir).unwrap();

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_games,
            get_recent_steam_user,
            import_screenshots
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
