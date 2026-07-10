use crate::AppRuntime;
use crate::app_dirs::PROJECT_DIRS;
use crate::steam::{initialize_steam, open_steam_section};
use atomic_float::AtomicF32;
use image::ImageReader;
use image::codecs::jpeg::JpegEncoder;
use image::imageops::{FilterType, resize};
use log::{error, info, warn};
use rayon::prelude::*;
use std::ffi::CString;
use std::fs::{File, create_dir_all, remove_dir_all};
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, atomic::Ordering};
use steamworks::sys::INVALID_SCREENSHOT_HANDLE;
use steamworks::sys::SteamAPI_ISteamScreenshots_AddScreenshotToLibrary as add_screenshot_to_library;
use steamworks::sys::SteamAPI_SteamScreenshots_v003 as get_steam_screenshots;
use tauri::Emitter;

const PROGRESS_EVENT: &str = "screenshotImportProgress";
const THUMB_WIDTH: u32 = steamworks::sys::k_ScreenshotThumbWidth as u32;
const MAX_SIDE: u32 = 16000;
const MAX_RESOLUTION: u32 = 26210175;

#[derive(Clone, Copy)]
struct ImportOptions {
    app_id: u32,
    jpeg_quality: u8,
    filter_type: FilterType,
}

struct ImportContext {
    window: tauri::Window<AppRuntime>,
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
#[specta::specta]
pub async fn import_screenshots(
    file_paths: Vec<String>,
    app_id: u32,
    jpeg_quality: u8,
    filter_type: String,
    window: tauri::Window<AppRuntime>,
) -> Result<(), String> {
    info!(
        "Importing {} screenshots under AppID {}",
        file_paths.len(),
        app_id
    );

    let num_of_files = file_paths.len();
    if num_of_files == 0 {
        warn!("Got no screenshots to import");
        return Err("No screenshots to import".to_string());
    }

    let options = ImportOptions {
        app_id,
        jpeg_quality: jpeg_quality.clamp(1, 100),
        filter_type: parse_filter_type(&filter_type),
    };

    // Check if steam is running and initialize client
    let client = initialize_steam(app_id)?;

    let ctx = Arc::new(ImportContext {
        window,
        cache_dir: PROJECT_DIRS.cache_dir().to_path_buf(),
        client: Mutex::new(client),
        screenshots_completed: AtomicF32::new(0.0),
        total_screenshots: num_of_files,
    });

    // Process screenshots in parallel
    let import_errors: Vec<String> = file_paths
        .par_iter()
        .filter_map(|file_path| import_single_screenshot(file_path, &ctx, options).err())
        .collect();

    info!("Emptying cache");
    let cleanup_result = remove_dir_all(&ctx.cache_dir)
        .and_then(|_| create_dir_all(&ctx.cache_dir))
        .map_err(|error| format!("Failed to empty screenshot cache: {error}"));

    let succeeded = num_of_files - import_errors.len();

    let open_section_result = if succeeded > 0 {
        info!(
            "Import of {} out of {} images complete, opening steam screenshots window",
            succeeded, num_of_files
        );
        open_steam_section(&format!("screenshots/{}", app_id))
    } else {
        Ok(())
    };

    if !import_errors.is_empty() {
        if let Err(error) = cleanup_result {
            error!("{error}");
        }
        if let Err(error) = open_section_result {
            error!("{error}");
        }

        let import_error = format_import_errors(&import_errors);
        error!("{import_error}");
        return Err(import_error);
    }

    cleanup_result?;
    open_section_result?;

    Ok(())
}

fn format_import_errors(errors: &[String]) -> String {
    match errors {
        [error] => error.clone(),
        errors => format!(
            "{} screenshots failed to import:\n{}",
            errors.len(),
            errors.join("\n")
        ),
    }
}

fn import_single_screenshot(
    file_path: &str,
    ctx: &ImportContext,
    options: ImportOptions,
) -> Result<(), String> {
    let mut progress_remaining = 1.0;
    let result = process_single_screenshot(file_path, ctx, options, &mut progress_remaining);

    if result.is_err() && progress_remaining > 0.0 {
        update_progress(
            &ctx.window,
            &ctx.screenshots_completed,
            ctx.total_screenshots,
            progress_remaining,
        );
    }

    result
}

fn process_single_screenshot(
    file_path: &str,
    ctx: &ImportContext,
    options: ImportOptions,
    progress_remaining: &mut f32,
) -> Result<(), String> {
    let img_path = Path::new(file_path);
    let img_name = img_path
        .file_stem()
        .and_then(|name| name.to_str())
        .ok_or_else(|| format!("Invalid screenshot path: {file_path}"))?;
    let extension = img_path
        .extension()
        .and_then(|extension| extension.to_str())
        .ok_or_else(|| format!("Screenshot has no valid extension: {file_path}"))?;

    let new_file_name = format!("{}_{}.jpg", img_name, options.app_id);
    let new_thumbnail_name = format!("{}_{}_thumb.jpg", img_name, options.app_id);

    info!("New file name: {}", new_file_name);

    // Load original image
    info!("Loading image: {}", img_path.display());
    let mut img = ImageReader::open(file_path)
        .map_err(|error| format!("Failed to open {img_name}.{extension}: {error}"))?
        .decode()
        .map_err(|error| format!("Failed to decode {img_name}.{extension}: {error}"))?;

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
            return Err(format!(
                "Failed to downscale {img_name}.{extension} to a valid size"
            ));
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
        let file = File::create(&new_img_path)
            .map_err(|error| format!("Failed to create {}: {error}", new_img_path.display()))?;
        let writer = BufWriter::new(file);
        let mut encoder = JpegEncoder::new_with_quality(writer, options.jpeg_quality);
        encoder
            .encode_image(&img)
            .map_err(|error| format!("Failed to encode {img_name}.{extension}: {error}"))?;
    } else {
        info!("Copying image {}.{}", img_name, extension);
        img.save(&new_img_path)
            .map_err(|error| format!("Failed to save {}: {error}", new_img_path.display()))?;
    }

    report_step_progress(ctx, progress_remaining, 0.3);

    // Create thumbnail image
    info!(
        "Resizing image {}.{} for thumbnail with {:?} q{}",
        img_name, extension, options.filter_type, options.jpeg_quality
    );
    let thumb_img_path = ctx.cache_dir.join(&new_thumbnail_name);

    let thumb_height = (THUMB_WIDTH * img.height()) / img.width();
    let thumb_img = resize(&img, THUMB_WIDTH, thumb_height, options.filter_type);
    let file = File::create(&thumb_img_path)
        .map_err(|error| format!("Failed to create {}: {error}", thumb_img_path.display()))?;
    let writer = BufWriter::new(&file);
    let mut encoder = JpegEncoder::new_with_quality(writer, options.jpeg_quality);
    encoder.encode_image(&thumb_img).map_err(|error| {
        format!("Failed to create thumbnail for {img_name}.{extension}: {error}")
    })?;

    report_step_progress(ctx, progress_remaining, 0.4);

    // Import screenshot
    info!(
        "Importing screenshot {} {}",
        new_img_path.display(),
        thumb_img_path.display()
    );
    unsafe {
        let screenshots = get_steam_screenshots();
        let screenshot_path = CString::new(new_img_path.to_string_lossy().as_bytes())
            .map_err(|error| format!("Invalid screenshot path: {error}"))?;
        let thumbnail_path = CString::new(thumb_img_path.to_string_lossy().as_bytes())
            .map_err(|error| format!("Invalid thumbnail path: {error}"))?;
        let width = img
            .width()
            .try_into()
            .map_err(|error| format!("Invalid screenshot width: {error}"))?;
        let height = img
            .height()
            .try_into()
            .map_err(|error| format!("Invalid screenshot height: {error}"))?;

        let screenshot_handle = add_screenshot_to_library(
            screenshots,
            screenshot_path.as_ptr(),
            thumbnail_path.as_ptr(),
            width,
            height,
        );

        if screenshot_handle == INVALID_SCREENSHOT_HANDLE {
            return Err(format!(
                "Steam failed to import {img_name}.{extension} into its screenshot library"
            ));
        }

        ctx.client
            .lock()
            .map_err(|error| format!("Failed to access Steam client: {error}"))?
            .run_callbacks();
    }
    info!("Import of {}.{} complete", img_name, extension);

    report_step_progress(ctx, progress_remaining, 0.3);

    Ok(())
}

fn report_step_progress(ctx: &ImportContext, progress_remaining: &mut f32, step_progress: f32) {
    update_progress(
        &ctx.window,
        &ctx.screenshots_completed,
        ctx.total_screenshots,
        step_progress,
    );
    *progress_remaining = (*progress_remaining - step_progress).max(0.0);
}

fn update_progress(
    window: &tauri::Window<AppRuntime>,
    screenshots_completed: &AtomicF32,
    total_screenshots: usize,
    step_progress: f32,
) {
    let completed = screenshots_completed.fetch_add(step_progress, Ordering::SeqCst);
    let progress = ((completed + step_progress) / total_screenshots as f32) * 100.0;
    if let Err(error) = window.emit(PROGRESS_EVENT, progress) {
        error!("Failed to emit screenshot import progress: {error}");
    }
}
