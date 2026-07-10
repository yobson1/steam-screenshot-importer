use crate::app_dirs::PROJECT_DIRS;
use crate::steam::{initialize_steam, open_steam_section};
use atomic_float::AtomicF32;
use image::ImageReader;
use image::codecs::jpeg::JpegEncoder;
use image::imageops::{FilterType, resize};
use log::{error, info, warn};
use rayon::prelude::*;
use std::ffi::CString;
use std::fs::{File, create_dir, remove_dir_all};
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, atomic::Ordering};
use std::thread;
use std::time::Duration;
use steamworks::sys::SteamAPI_ISteamScreenshots_AddScreenshotToLibrary as add_screenshot_to_library;
use steamworks::sys::SteamAPI_SteamScreenshots_v003 as get_steam_screenshots;
use tauri::Emitter;

const PROGRESS_EVENT: &str = "screenshotImportProgress";
const ERROR_EVENT: &str = "screenshotImportError";
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
    window: tauri::Window<R>,
    cache_dir: PathBuf,
    client: Mutex<steamworks::Client>,
    screenshots_completed: AtomicF32,
    total_screenshots: usize,
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
pub async fn import_screenshots<R: tauri::Runtime>(
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

    let ctx = Arc::new(ImportContext {
        window,
        cache_dir: PROJECT_DIRS.cache_dir().to_path_buf(),
        client: Mutex::new(client),
        screenshots_completed: AtomicF32::new(0.0),
        total_screenshots: num_of_files,
    });

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
    remove_dir_all(&ctx.cache_dir)
        .and_then(|_| create_dir(&ctx.cache_dir))
        .unwrap();

    String::default()
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
