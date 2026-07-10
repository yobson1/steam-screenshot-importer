use crate::AppRuntime;
use crate::app_dirs::PROJECT_DIRS;
use crate::steam::{initialize_steam, open_steam_section};
use atomic_float::AtomicF32;
use image::codecs::jpeg::JpegEncoder;
use image::imageops::{FilterType as ImageFilterType, resize};
use image::{DynamicImage, ImageReader};
use log::{error, info, warn};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::ffi::CString;
use std::fs::{File, copy, create_dir_all, remove_dir_all};
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, atomic::Ordering};
use steamworks::sys::INVALID_SCREENSHOT_HANDLE;
use steamworks::sys::SteamAPI_ISteamScreenshots_AddScreenshotToLibrary as add_screenshot_to_library;
use steamworks::sys::SteamAPI_SteamScreenshots_v003 as get_steam_screenshots;
use tauri::Emitter;

const PROGRESS_EVENT: &str = "screenshotImportProgress";
const THUMB_WIDTH: u32 = steamworks::sys::k_ScreenshotThumbWidth as u32;
const MAX_SIDE: u32 = 16_000;
const MAX_RESOLUTION: u32 = 26_210_175;

#[derive(Clone, Copy, Deserialize, specta::Type)]
pub enum ResizeFilterType {
    Nearest,
    Triangle,
    CatmullRom,
    Gaussian,
    Lanczos3,
}

impl From<ResizeFilterType> for ImageFilterType {
    fn from(filter_type: ResizeFilterType) -> Self {
        match filter_type {
            ResizeFilterType::Nearest => Self::Nearest,
            ResizeFilterType::Triangle => Self::Triangle,
            ResizeFilterType::CatmullRom => Self::CatmullRom,
            ResizeFilterType::Gaussian => Self::Gaussian,
            ResizeFilterType::Lanczos3 => Self::Lanczos3,
        }
    }
}

#[derive(Clone, Copy)]
struct ImportOptions {
    app_id: u32,
    jpeg_quality: u8,
    filter_type: ImageFilterType,
}

#[derive(Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ImportError {
    summary: String,
    errors: Vec<ImportFailure>,
}

#[derive(Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
struct ImportFailure {
    file_path: String,
    message: String,
}

impl ImportError {
    fn from_failures(total: usize, errors: Vec<ImportFailure>) -> Self {
        let failed = errors.len();
        let summary = if failed == total {
            format!("All {total} screenshots failed to import.")
        } else {
            format!("{failed} of {total} screenshots failed to import.")
        };

        Self { summary, errors }
    }
}

impl From<String> for ImportError {
    fn from(summary: String) -> Self {
        Self {
            summary,
            errors: Vec::new(),
        }
    }
}

struct ImportContext {
    window: tauri::Window<AppRuntime>,
    cache_dir: PathBuf,
    client: Mutex<steamworks::Client>,
    screenshots_completed: AtomicF32,
    total_screenshots: usize,
}

#[tauri::command]
#[specta::specta]
pub async fn import_screenshots(
    file_paths: Vec<String>,
    app_id: u32,
    jpeg_quality: u8,
    filter_type: ResizeFilterType,
    window: tauri::Window<AppRuntime>,
) -> Result<(), ImportError> {
    info!(
        "Importing {} screenshots under AppID {}",
        file_paths.len(),
        app_id
    );

    let num_of_files = file_paths.len();
    if num_of_files == 0 {
        warn!("Got no screenshots to import");
        return Err("No screenshots to import".to_string().into());
    }

    let options = ImportOptions {
        app_id,
        jpeg_quality: jpeg_quality.clamp(1, 100),
        filter_type: filter_type.into(),
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
    let import_errors: Vec<ImportFailure> = file_paths
        .par_iter()
        .filter_map(|file_path| import_single_screenshot(file_path, &ctx, options).err())
        .collect();

    info!("Emptying cache");
    let cleanup_result = remove_dir_all(&ctx.cache_dir)
        .and_then(|()| create_dir_all(&ctx.cache_dir))
        .map_err(|error| format!("Failed to empty screenshot cache: {error}"));

    let succeeded = num_of_files - import_errors.len();

    let open_section_result = if succeeded > 0 {
        info!(
            "Import of {succeeded} out of {num_of_files} images complete, opening steam screenshots window"
        );
        open_steam_section(&format!("screenshots/{app_id}"))
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

        for import_error in &import_errors {
            error!(
                "Failed to import {}: {}",
                import_error.file_path, import_error.message
            );
        }

        return Err(ImportError::from_failures(num_of_files, import_errors));
    }

    cleanup_result?;
    open_section_result?;

    Ok(())
}

fn import_single_screenshot(
    file_path: &str,
    ctx: &ImportContext,
    options: ImportOptions,
) -> Result<(), ImportFailure> {
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

    result.map_err(|message| ImportFailure {
        file_path: file_path.to_string(),
        message,
    })
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

    info!("New file name: {new_file_name}");

    // Load original image
    info!("Loading image: {}", img_path.display());
    let img = ImageReader::open(file_path)
        .map_err(|error| format!("Failed to open {img_name}.{extension}: {error}"))?
        .decode()
        .map_err(|error| format!("Failed to decode {img_name}.{extension}: {error}"))?;

    // Convert to jpg or downscale if needed
    let new_img_path = ctx.cache_dir.join(&new_file_name);

    let (img, was_resized) = resize_for_steam(img, img_name, extension, options);
    let is_jpeg = extension.eq_ignore_ascii_case("jpg") || extension.eq_ignore_ascii_case("jpeg");

    if is_jpeg && !was_resized {
        info!("Copying image {img_name}.{extension}");
        copy(img_path, &new_img_path)
            .map_err(|error| format!("Failed to copy {}: {error}", new_img_path.display()))?;
    } else {
        info!(
            "Encoding image {img_name}.{extension} as jpg with {:?} q{}",
            options.filter_type, options.jpeg_quality
        );
        let file = File::create(&new_img_path)
            .map_err(|error| format!("Failed to create {}: {error}", new_img_path.display()))?;
        let writer = BufWriter::new(file);
        let mut encoder = JpegEncoder::new_with_quality(writer, options.jpeg_quality);
        encoder
            .encode_image(&img)
            .map_err(|error| format!("Failed to encode {img_name}.{extension}: {error}"))?;
    }

    report_step_progress(ctx, progress_remaining, 0.3);

    // Create thumbnail image
    info!(
        "Resizing image {img_name}.{extension} for thumbnail with {:?} q{}",
        options.filter_type, options.jpeg_quality
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
    info!("Import of {img_name}.{extension} complete");

    report_step_progress(ctx, progress_remaining, 0.3);

    Ok(())
}

fn resize_for_steam(
    img: DynamicImage,
    img_name: &str,
    extension: &str,
    options: ImportOptions,
) -> (DynamicImage, bool) {
    if img.width() <= MAX_SIDE
        && img.height() <= MAX_SIDE
        && img.width() * img.height() <= MAX_RESOLUTION
    {
        return (img, false);
    }

    warn!(
        "Image {img_name}.{extension} is too large to be imported, it will be downscaled with {:?} q{}",
        options.filter_type, options.jpeg_quality
    );

    let (new_width, new_height) = downscaled_dimensions(img.width(), img.height());
    let img = img.resize_exact(new_width, new_height, options.filter_type);

    info!("{img_name}.{extension} new size: {new_width}x{new_height}");
    (img, true)
}

fn downscaled_dimensions(width: u32, height: u32) -> (u32, u32) {
    let (dominant, minor, width_is_dominant) = if width >= height {
        (width, height, true)
    } else {
        (height, width, false)
    };
    let mut lower = 1;
    let mut upper = dominant.min(MAX_SIDE);

    while lower < upper {
        let candidate = lower + (upper - lower).div_ceil(2);
        let scaled_minor = (u64::from(minor) * u64::from(candidate) / u64::from(dominant)).max(1);
        if u64::from(candidate) * scaled_minor <= u64::from(MAX_RESOLUTION) {
            lower = candidate;
        } else {
            upper = candidate - 1;
        }
    }

    let scaled_minor = u64::from(minor) * u64::from(lower) / u64::from(dominant);
    let scaled_minor = u32::try_from(scaled_minor.max(1))
        .expect("scaled minor image dimension cannot exceed the dominant dimension");
    if width_is_dominant {
        (lower, scaled_minor)
    } else {
        (scaled_minor, lower)
    }
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
    #[allow(clippy::cast_precision_loss)]
    let progress = ((completed + step_progress) / total_screenshots as f32) * 100.0;
    if let Err(error) = window.emit(PROGRESS_EVENT, progress) {
        error!("Failed to emit screenshot import progress: {error}");
    }
}
