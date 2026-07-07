use serde::Deserialize;
#[cfg(unix)]
use std::os::unix::fs::symlink;
#[cfg(windows)]
use std::os::windows::fs::symlink_file;
use std::{
    fs, io,
    path::{Path, PathBuf},
};

#[derive(Deserialize)]
struct CargoLock {
    package: Vec<Package>,
}

#[derive(Deserialize)]
struct Package {
    name: String,
    version: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var_os("NO_STEAMWORKS").is_some() {
        println!("NO_STEAMWORKS set, skipping Steamworks binary fetch");
        return Ok(());
    }

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let lock_path = root.join("Cargo.lock");
    let lock = fs::read_to_string(lock_path)?;
    let lock: CargoLock = toml::from_str(&lock)?;

    let version = lock
        .package
        .iter()
        .find(|p| p.name == "steamworks")
        .map(|p| p.version.as_str())
        .expect("steamworks not found in Cargo.lock");

    println!("steamworks version: {version}");

    let steam_dir = root.join("steam_api");
    let version_dir = steam_dir.join(version);

    fs::create_dir_all(&version_dir)?;

    download_if_missing(
        &version_dir.join("steam_api64.dll"),
        &format!(
            "https://github.com/Noxime/steamworks-rs/raw/refs/tags/v{version}/steamworks-sys/lib/steam/redistributable_bin/win64/steam_api64.dll"
        ),
    )?;

    download_if_missing(
        &version_dir.join("libsteam_api.so"),
        &format!(
            "https://github.com/Noxime/steamworks-rs/raw/refs/tags/v{version}/steamworks-sys/lib/steam/redistributable_bin/linux64/libsteam_api.so"
        ),
    )?;

    update_link(
        &root.join("steam_api64.dll"),
        &version_dir.join("steam_api64.dll"),
    )?;

    update_link(
        &root.join("libsteam_api.so"),
        &version_dir.join("libsteam_api.so"),
    )?;

    Ok(())
}

fn download_if_missing(path: &Path, url: &str) -> Result<(), Box<dyn std::error::Error>> {
    if path.exists() {
        println!("exists: {}", path.display());
        return Ok(());
    }

    println!("downloading {}", url);

    let response = ureq::get(url).call()?;

    let mut file = fs::File::create(path)?;
    let mut reader = response.into_body().into_reader();

    io::copy(&mut reader, &mut file)?;

    Ok(())
}

fn update_link(link: &Path, target: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if link.exists() || link.symlink_metadata().is_ok() {
        fs::remove_file(link)?;
    }

    #[cfg(unix)]
    symlink(target, link)?;
    #[cfg(windows)]
    symlink_file(target, link)?;

    println!("{} -> {}", link.display(), target.display());

    Ok(())
}
