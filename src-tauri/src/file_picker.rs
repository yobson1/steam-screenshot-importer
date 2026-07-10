#[tauri::command]
#[specta::specta]
pub fn pick_screenshot_files() -> Vec<String> {
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
