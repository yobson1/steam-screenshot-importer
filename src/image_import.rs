use crate::app_dirs::PROJECT_DIRS;
use crate::preferences::ResizeFilter;
use crate::steam::{initialize_steam, open_steam_section};
use image::codecs::jpeg::JpegEncoder;
use image::imageops::{FilterType as ImageFilterType, resize};
use image::{DynamicImage, ImageReader};
use log::{error, info, warn};
use rayon::prelude::*;
use std::ffi::CString;
use std::fmt;
use std::fs::{File, copy, create_dir_all, remove_dir_all};
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};
use steamworks::sys::INVALID_SCREENSHOT_HANDLE;
use steamworks::sys::SteamAPI_ISteamScreenshots_AddScreenshotToLibrary as add_screenshot_to_library;
use steamworks::sys::SteamAPI_SteamScreenshots_v003 as get_steam_screenshots;

const THUMB_WIDTH: u32 = steamworks::sys::k_ScreenshotThumbWidth as u32;
const MAX_SIDE: u32 = 16_000;
const MAX_RESOLUTION: u32 = 26_210_175;
const PROGRESS_UNITS_PER_SCREENSHOT: usize = 10;

impl From<ResizeFilter> for ImageFilterType {
    fn from(filter_type: ResizeFilter) -> Self {
        match filter_type {
            ResizeFilter::Nearest => Self::Nearest,
            ResizeFilter::Triangle => Self::Triangle,
            ResizeFilter::CatmullRom => Self::CatmullRom,
            ResizeFilter::Gaussian => Self::Gaussian,
            ResizeFilter::Lanczos3 => Self::Lanczos3,
        }
    }
}

#[derive(Clone, Copy)]
struct ImportOptions {
    app_id: u32,
    jpeg_quality: u8,
    filter_type: ImageFilterType,
}

#[derive(Debug)]
pub struct ImportError {
    pub summary: String,
    pub errors: Vec<ImportFailure>,
}

#[derive(Debug)]
pub struct ImportFailure {
    pub file_path: PathBuf,
    pub message: String,
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

impl fmt::Display for ImportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.summary)
    }
}

impl std::error::Error for ImportError {}

struct ImportContext {
    cache_dir: PathBuf,
    client: Mutex<steamworks::Client>,
    progress_units_completed: AtomicUsize,
    total_screenshots: usize,
    report_progress: Arc<dyn Fn(f32) + Send + Sync>,
}

pub fn import_screenshots(
    file_paths: &[PathBuf],
    app_id: u32,
    jpeg_quality: u8,
    filter_type: ResizeFilter,
    report_progress: impl Fn(f32) + Send + Sync + 'static,
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
    let cache_dir = PROJECT_DIRS.cache_dir().to_path_buf();
    create_dir_all(&cache_dir)
        .map_err(|error| format!("Failed to create screenshot cache: {error}"))?;

    let ctx = Arc::new(ImportContext {
        cache_dir,
        client: Mutex::new(client),
        progress_units_completed: AtomicUsize::new(0),
        total_screenshots: num_of_files,
        report_progress: Arc::new(report_progress),
    });

    // Process screenshots in parallel
    let import_errors: Vec<ImportFailure> = file_paths
        .par_iter()
        .enumerate()
        .filter_map(|(file_index, file_path)| {
            import_single_screenshot(file_path, file_index, &ctx, options).err()
        })
        .collect();

    info!("Emptying cache");
    let cleanup_result = remove_dir_all(&ctx.cache_dir)
        .or_else(|error| {
            (error.kind() == std::io::ErrorKind::NotFound)
                .then_some(())
                .ok_or(error)
        })
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
                import_error.file_path.display(),
                import_error.message
            );
        }

        return Err(ImportError::from_failures(num_of_files, import_errors));
    }

    cleanup_result?;
    open_section_result?;

    Ok(())
}

fn import_single_screenshot(
    file_path: &Path,
    file_index: usize,
    ctx: &ImportContext,
    options: ImportOptions,
) -> Result<(), ImportFailure> {
    let mut progress_remaining = PROGRESS_UNITS_PER_SCREENSHOT;
    let result =
        process_single_screenshot(file_path, file_index, ctx, options, &mut progress_remaining);

    if result.is_err() && progress_remaining > 0 {
        update_progress(ctx, progress_remaining);
    }

    result.map_err(|message| ImportFailure {
        file_path: file_path.to_path_buf(),
        message,
    })
}

fn process_single_screenshot(
    img_path: &Path,
    file_index: usize,
    ctx: &ImportContext,
    options: ImportOptions,
    progress_remaining: &mut usize,
) -> Result<(), String> {
    let img_name = img_path
        .file_stem()
        .and_then(|name| name.to_str())
        .ok_or_else(|| format!("Invalid screenshot path: {}", img_path.display()))?;
    let extension = img_path
        .extension()
        .and_then(|extension| extension.to_str())
        .ok_or_else(|| format!("Screenshot has no valid extension: {}", img_path.display()))?;

    let new_file_name = format!("{}_{}.jpg", img_name, options.app_id);
    let new_thumbnail_name = format!("{}_{}_thumb.jpg", img_name, options.app_id);

    info!("New file name: {new_file_name}");

    // Load original image
    info!("Loading image: {}", img_path.display());
    let img = ImageReader::open(img_path)
        .map_err(|error| format!("Failed to open {img_name}.{extension}: {error}"))?
        .decode()
        .map_err(|error| format!("Failed to decode {img_name}.{extension}: {error}"))?;

    // Convert to jpg or downscale if needed
    let file_cache_dir = ctx.cache_dir.join(file_index.to_string());
    create_dir_all(&file_cache_dir)
        .map_err(|error| format!("Failed to create screenshot cache: {error}"))?;
    let new_img_path = file_cache_dir.join(&new_file_name);

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

    report_step_progress(ctx, progress_remaining, 3);

    // Create thumbnail image
    info!(
        "Resizing image {img_name}.{extension} for thumbnail with {:?} q{}",
        options.filter_type, options.jpeg_quality
    );
    let thumb_img_path = file_cache_dir.join(&new_thumbnail_name);

    let thumb_height =
        (u64::from(THUMB_WIDTH) * u64::from(img.height()) / u64::from(img.width())).max(1);
    let thumb_height = u32::try_from(thumb_height)
        .map_err(|error| format!("Invalid thumbnail height: {error}"))?;
    let thumb_img = resize(&img, THUMB_WIDTH, thumb_height, options.filter_type);
    let file = File::create(&thumb_img_path)
        .map_err(|error| format!("Failed to create {}: {error}", thumb_img_path.display()))?;
    let writer = BufWriter::new(&file);
    let mut encoder = JpegEncoder::new_with_quality(writer, options.jpeg_quality);
    encoder.encode_image(&thumb_img).map_err(|error| {
        format!("Failed to create thumbnail for {img_name}.{extension}: {error}")
    })?;

    report_step_progress(ctx, progress_remaining, 4);

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

    report_step_progress(ctx, progress_remaining, 3);

    Ok(())
}

fn report_step_progress(ctx: &ImportContext, progress_remaining: &mut usize, step_progress: usize) {
    update_progress(ctx, step_progress);
    *progress_remaining = progress_remaining.saturating_sub(step_progress);
}

fn update_progress(ctx: &ImportContext, step_progress: usize) {
    let completed = ctx
        .progress_units_completed
        .fetch_add(step_progress, Ordering::Relaxed);
    #[allow(clippy::cast_precision_loss)]
    let progress = ((completed + step_progress) as f32
        / (ctx.total_screenshots as f32 * PROGRESS_UNITS_PER_SCREENSHOT as f32))
        * 100.0;
    (ctx.report_progress)(progress.clamp(0.0, 100.0));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn downscaled_images_fit_steam_limits() {
        for (width, height) in [
            (20_000, 100),
            (100, 20_000),
            (20_000, 20_000),
            (80_000, 10_000),
        ] {
            let (scaled_width, scaled_height) = downscaled_dimensions(width, height);
            assert!(scaled_width <= MAX_SIDE);
            assert!(scaled_height <= MAX_SIDE);
            assert!(
                u64::from(scaled_width) * u64::from(scaled_height) <= u64::from(MAX_RESOLUTION)
            );
            assert!(scaled_width > 0);
            assert!(scaled_height > 0);
        }
    }

    #[test]
    fn import_error_summary_distinguishes_partial_and_total_failure() {
        let failure = || ImportFailure {
            file_path: PathBuf::from("screenshot.png"),
            message: "failed".to_owned(),
        };

        assert_eq!(
            ImportError::from_failures(2, vec![failure()]).summary,
            "1 of 2 screenshots failed to import."
        );
        assert_eq!(
            ImportError::from_failures(1, vec![failure()]).summary,
            "All 1 screenshots failed to import."
        );
    }
}
