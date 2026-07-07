use atomic_float::AtomicF32;
use directories::ProjectDirs;
use image::ImageReader;
use image::codecs::jpeg::JpegEncoder;
use image::imageops::{FilterType, resize};
use lazy_static::lazy_static;
use log::{error, info, warn};
use rayon::prelude::*;
use serde_json::Value;
use simple_logger::SimpleLogger;
use std::collections::HashMap;
use std::ffi::CString;
use std::fs::{File, create_dir, create_dir_all, read, remove_dir_all};
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;
use std::sync::{Arc, Mutex, atomic::Ordering};
use std::thread;
use std::time::{Duration, Instant};
use steamlocate::{SteamApp, SteamDir};
use steamworks::Client;
use steamworks::sys::SteamAPI_ISteamScreenshots_AddScreenshotToLibrary as add_screenshot_to_library;
use steamworks::sys::SteamAPI_IsSteamRunning as is_steam_running;
use steamworks::sys::SteamAPI_SteamScreenshots_v003 as get_steam_screenshots;
use steamy_vdf as vdf;
use tauri::Emitter;
use walkdir::WalkDir;

static HTTP_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
const LIB_CACHE_PATH: &str = "appcache/librarycache/";
const PROGRESS_EVENT: &str = "screenshotImportProgress";
const ERROR_EVENT: &str = "screenshotImportError";

lazy_static! {
    static ref PROJECT_DIRS: ProjectDirs = ProjectDirs::from("com", "yob", "ssi").unwrap();
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
        .unwrap()
        .wait()
        .unwrap();
}

#[tauri::command]
fn pick_screenshot_files() -> Vec<String> {
    let default_dir =
        directories::UserDirs::new().and_then(|dirs| dirs.picture_dir().map(|p| p.to_path_buf()));

    let mut dialog = rfd::FileDialog::new()
        .set_title("Select screenshots to import")
        .add_filter(
            "Images",
            &[
                "png", "jpg", "jpeg", "bmp", "ico", "tiff", "tif", "webp", "avif", "pnm", "dds",
                "tga", "exr",
            ],
        );

    if let Some(dir) = default_dir {
        dialog = dialog.set_directory(dir);
    }

    dialog
        .pick_files()
        .unwrap_or_default()
        .into_iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect()
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
fn get_games() -> Result<Vec<(u32, String, String)>, String> {
    let mut steamdir: SteamDir = SteamDir::locate().ok_or("Failed to locate Steam installation")?;
    let apps_hash: HashMap<u32, Option<SteamApp>> = steamdir.apps().clone();
    let apps: Vec<u32> = apps_hash.keys().cloned().collect();
    let steam_path: PathBuf = steamdir.path;

    let mut imgs: Vec<(u32, String, String)> = vec![];
    for appid in apps {
        let img = find_library_capsule(&steam_path, appid)
            .and_then(|path| {
                info!("Found image path: {}", path.display());
                read(path).ok()
            })
            .unwrap_or_default();

        let b64_img = base64::encode(img);

        let app = apps_hash.get(&appid).unwrap().as_ref().unwrap();

        let name = app.name.as_ref().unwrap();

        imgs.push((appid, b64_img, name.to_string()));
    }

    Ok(imgs)
}

fn http_client() -> &'static reqwest::Client {
    HTTP_CLIENT.get_or_init(reqwest::Client::new)
}

#[tauri::command]
async fn get_library_image(app_id: u32) -> Option<String> {
    let input = serde_json::json!({
        "ids": [
            {
                "appid": app_id
            }
        ],
        "context": {
            "language": "english",
            "country_code": "US"
        },
        "data_request": {
            "include_assets": true
        }
    });

    let response = http_client()
        .get("https://api.steampowered.com/IStoreBrowseService/GetItems/v1/")
        .query(&[("input_json", input.to_string())])
        .send()
        .await
        .ok()?
        .text()
        .await
        .ok()?;

    let json: Value = serde_json::from_str(&response).ok()?;

    let assets = json
        .get("response")?
        .get("store_items")?
        .get(0)?
        .get("assets")?;

    let format = assets.get("asset_url_format")?.as_str()?;

    let capsule = assets.get("library_capsule")?.as_str()?;

    let url = format!(
        "https://shared.fastly.steamstatic.com/store_item_assets/{}",
        format.replace("${FILENAME}", capsule)
    );

    info!("Resolved URL for AppID {}: {}", app_id, url);

    Some(url)
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

const THUMB_WIDTH: u32 = steamworks::sys::k_ScreenshotThumbWidth as u32;
const MAX_SIDE: u32 = 16000;
const MAX_RESOLUTION: u32 = 26210175;

#[derive(Clone, Copy)]
struct ImportOptions {
    app_id: u32,
    jpeg_quality: u8,
    filter_type: FilterType,
}

struct ImportContext<R: tauri::Runtime> {
    window: Arc<tauri::Window<R>>,
    cache_dir: Arc<PathBuf>,
    client: Arc<Mutex<steamworks::Client>>,
    screenshots_completed: Arc<AtomicF32>,
    total_screenshots: usize,
}

impl<R: tauri::Runtime> Clone for ImportContext<R> {
    fn clone(&self) -> Self {
        Self {
            window: Arc::clone(&self.window),
            cache_dir: Arc::clone(&self.cache_dir),
            client: Arc::clone(&self.client),
            screenshots_completed: Arc::clone(&self.screenshots_completed),
            total_screenshots: self.total_screenshots,
        }
    }
}

fn parse_filter_type(s: &str) -> FilterType {
    match s {
        "Nearest" => FilterType::Nearest,
        "Triangle" => FilterType::Triangle,
        "CatmullRom" => FilterType::CatmullRom,
        "Gaussian" => FilterType::Gaussian,
        _ => FilterType::Lanczos3,
    }
}

#[tauri::command]
async fn import_screenshots<R: tauri::Runtime>(
    file_paths: Vec<String>,
    app_id: u32,
    jpeg_quality: u8,
    filter_type: String,
    window: tauri::Window<R>,
) -> String {
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

    let options = ImportOptions {
        app_id,
        jpeg_quality: jpeg_quality.clamp(1, 100),
        filter_type: parse_filter_type(&filter_type),
    };

    // Check if steam is running and initialize client
    let client = initialize_steam(app_id).unwrap();

    let ctx = ImportContext {
        window: Arc::new(window),
        cache_dir: Arc::new(PROJECT_DIRS.cache_dir().to_path_buf()),
        client: Arc::new(Mutex::new(client)),
        screenshots_completed: Arc::new(AtomicF32::new(0.0)),
        total_screenshots: num_of_files,
    };

    // Process screenshots in parallel
    file_paths.par_iter().for_each(|file_path| {
        let ctx = ctx.clone();
        import_single_screenshot(file_path, &ctx, options);
    });

    info!(
        "Import of {} images complete, opening steam screenshots window",
        num_of_files
    );

    // Open the steam screenshots window for upload
    open_steam_section(&format!("screenshots/{}", app_id));

    info!("Emptying cache");
    remove_dir_all(&*ctx.cache_dir)
        .and_then(|_| create_dir(&*ctx.cache_dir))
        .unwrap();

    drop(ctx);

    String::default()
}

fn initialize_steam(app_id: u32) -> Result<Client, String> {
    if unsafe { is_steam_running() } {
        Client::init_app(app_id).map_err(|_| "Failed to initialize steamworks!\nMake sure steam is open and you own the game you're attempting to import for.".to_string())
    } else {
        open_steam_section("main");
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

fn import_single_screenshot<R: tauri::Runtime>(
    file_path: &str,
    ctx: &ImportContext<R>,
    options: ImportOptions,
) {
    let img_path = Path::new(file_path);
    let img_name = img_path.file_stem().unwrap().to_str().unwrap();
    let extension = img_path.extension().unwrap().to_str().unwrap();

    let new_file_name = format!("{}_{}.jpg", img_name, options.app_id);
    let new_thumbnail_name = format!("{}_{}_thumb.jpg", img_name, options.app_id);

    info!("New file name: {}", new_file_name);

    // Load original image
    info!("Loading image: {}", img_path.display());
    let mut img = match ImageReader::open(file_path).unwrap().decode() {
        Ok(img) => img,
        Err(e) => {
            ctx.window
                .emit(ERROR_EVENT, &format!("{}.{}\n{}", img_name, extension, e))
                .unwrap();
            error!("{}", e);
            thread::sleep(Duration::from_millis(2500));
            return;
        }
    };

    // Convert to jpg or downscale if needed
    let new_img_path = ctx.cache_dir.join(&new_file_name);

    if img.width() > MAX_SIDE
        || img.height() > MAX_SIDE
        || img.width() * img.height() > MAX_RESOLUTION
    {
        warn!(
            "Image {}.{} is too large to be imported, it will be downscaled with {:?} q{}",
            img_name, extension, options.filter_type, options.jpeg_quality
        );

        let scale_factor = f32::min(
            MAX_SIDE as f32 / f32::max(img.width() as f32, img.height() as f32),
            MAX_RESOLUTION as f32 / (img.width() * img.height()) as f32,
        );
        let new_width = (img.width() as f32 * scale_factor) as u32;
        let new_height = (img.height() as f32 * scale_factor) as u32;

        if new_width == 0 || new_height == 0 {
            warn!(
                "Image {}.{} is too large to be imported and cannot be downscaled correctly, it will be skipped",
                img_name, extension
            );
            return;
        }

        img = img.resize_exact(new_width, new_height, options.filter_type);

        info!(
            "{}.{} new size: {}x{}",
            img_name, extension, new_width, new_height
        );
    }

    if extension != "jpg" && extension != "jpeg" {
        info!(
            "Converting image {}.{} to jpg with {:?} q{}",
            img_name, extension, options.filter_type, options.jpeg_quality
        );
        let file = File::create(&new_img_path).unwrap();
        let writer = BufWriter::new(file);
        let mut encoder = JpegEncoder::new_with_quality(writer, options.jpeg_quality);
        encoder.encode_image(&img).unwrap();
    } else {
        info!("Copying image {}.{}", img_name, extension);
        img.save(&new_img_path).unwrap();
    }

    update_progress(
        &ctx.window,
        &ctx.screenshots_completed,
        ctx.total_screenshots,
        0.3,
    );

    // Create thumbnail image
    info!(
        "Resizing image {}.{} for thumbnail with {:?} q{}",
        img_name, extension, options.filter_type, options.jpeg_quality
    );
    let thumb_img_path = ctx.cache_dir.join(&new_thumbnail_name);

    let thumb_height = (THUMB_WIDTH * img.height()) / img.width();
    let thumb_img = resize(&img, THUMB_WIDTH, thumb_height, options.filter_type);
    let file = File::create(&thumb_img_path).unwrap();
    let writer = BufWriter::new(&file);
    let mut encoder = JpegEncoder::new_with_quality(writer, options.jpeg_quality);
    encoder.encode_image(&thumb_img).unwrap();

    update_progress(
        &ctx.window,
        &ctx.screenshots_completed,
        ctx.total_screenshots,
        0.4,
    );

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
        ctx.client.lock().unwrap().run_callbacks();
    }
    info!("Import of {}.{} complete", img_name, extension);

    update_progress(
        &ctx.window,
        &ctx.screenshots_completed,
        ctx.total_screenshots,
        0.3,
    );
}

fn update_progress<R: tauri::Runtime>(
    window: &tauri::Window<R>,
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
    create_dir_all(cache_dir).unwrap();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_games,
            get_recent_steam_user,
            import_screenshots,
            pick_screenshot_files,
            get_library_image
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
