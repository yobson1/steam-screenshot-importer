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
    println!("cargo:rerun-if-env-changed=NO_STEAMWORKS");
    println!("cargo:rerun-if-changed=Cargo.lock");

    fetch_steamworks()?;
    tauri_build::build();

    Ok(())
}

fn fetch_steamworks() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var_os("NO_STEAMWORKS").is_some() {
        println!("cargo:warning=NO_STEAMWORKS set, skipping Steamworks binary fetch");
        return Ok(());
    }

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let lock = fs::read_to_string(root.join("Cargo.lock"))?;
    let lock: CargoLock = toml::from_str(&lock)?;

    let version = lock
        .package
        .iter()
        .find(|package| package.name == "steamworks")
        .map(|package| package.version.as_str())
        .expect("steamworks not found in Cargo.lock");

    println!("Steamworks version: {version}");

    let version_dir = root.join("steam_api").join(version);
    fs::create_dir_all(&version_dir)?;

    let dll = version_dir.join("steam_api64.dll");
    download_if_missing(
        &dll,
        &format!(
            "https://github.com/Noxime/steamworks-rs/raw/refs/tags/v{version}/steamworks-sys/lib/steam/redistributable_bin/win64/steam_api64.dll"
        ),
    )?;

    let so = version_dir.join("libsteam_api.so");
    download_if_missing(
        &so,
        &format!(
            "https://github.com/Noxime/steamworks-rs/raw/refs/tags/v{version}/steamworks-sys/lib/steam/redistributable_bin/linux64/libsteam_api.so"
        ),
    )?;

    update_link(&root.join("steam_api64.dll"), &dll)?;
    update_link(&root.join("libsteam_api.so"), &so)?;

    Ok(())
}

fn download_if_missing(path: &Path, url: &str) -> Result<(), Box<dyn std::error::Error>> {
    if path.exists() {
        println!("Exists: {}", path.display());
        return Ok(());
    }

    println!("Downloading {url}");

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
