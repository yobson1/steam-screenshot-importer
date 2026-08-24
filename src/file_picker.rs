use std::path::PathBuf;

pub async fn pick_screenshot_files() -> Vec<PathBuf> {
    let default_dir = directories::UserDirs::new()
        .and_then(|dirs| dirs.picture_dir().map(std::path::Path::to_path_buf));

    let mut dialog = rfd::AsyncFileDialog::new()
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
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|file| file.path().to_path_buf())
        .collect()
}
