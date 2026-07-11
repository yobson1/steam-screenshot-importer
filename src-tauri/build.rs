use serde::Deserialize;
#[cfg(unix)]
use std::os::unix::fs::symlink;
#[cfg(windows)]
use std::os::windows::fs::symlink_file;
use std::{
    fs, io,
    path::{Path, PathBuf},
};

const STEAMWORKS_BASE_URL: &str = "https://github.com/Noxime/steamworks-rs/raw/refs/tags";
const STEAMWORKS_LIBRARIES: &[(&str, &str, &str)] = &[
    ("windows", "x86", "steam_api.dll"),
    ("windows", "x86_64", "win64/steam_api64.dll"),
    ("linux", "x86", "linux32/libsteam_api.so"),
    ("linux", "x86_64", "linux64/libsteam_api.so"),
    ("linux", "aarch64", "linuxarm64/libsteam_api.so"),
    ("android", "aarch64", "androidarm64/libsteam_api.so"),
    ("macos", "x86_64", "osx/libsteam_api.dylib"),
    ("macos", "aarch64", "osx/libsteam_api.dylib"),
];

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

    if std::env::var_os("NO_STEAMWORKS").is_some() {
        println!("cargo:warning=NO_STEAMWORKS set, skipping Steamworks binary fetch");
        tauri_build::build();
        return Ok(());
    }

    let target_os = std::env::var("CARGO_CFG_TARGET_OS")?;
    let target_arch = std::env::var("CARGO_CFG_TARGET_ARCH")?;
    let remote_path = steamworks_remote_path(&target_os, &target_arch)?;
    let library_name = library_name(remote_path)?;

    fetch_steamworks(remote_path, library_name, &target_os, &target_arch)?;
    tauri_build::build();

    Ok(())
}

fn steamworks_remote_path(
    target_os: &str,
    target_arch: &str,
) -> Result<&'static str, Box<dyn std::error::Error>> {
    STEAMWORKS_LIBRARIES
        .iter()
        .find_map(|(os, arch, path)| (*os == target_os && *arch == target_arch).then_some(*path))
        .ok_or_else(|| format!("unsupported Steamworks target: {target_arch}-{target_os}").into())
}

fn steamworks_library_path(
    root: &Path,
    version: &str,
    target_os: &str,
    target_arch: &str,
    library_name: &str,
) -> PathBuf {
    root.join("steam_api")
        .join(version)
        .join(target_os)
        .join(target_arch)
        .join(library_name)
}

fn library_name(remote_path: &str) -> Result<&str, Box<dyn std::error::Error>> {
    Path::new(remote_path)
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| format!("invalid Steamworks remote path: {remote_path}").into())
}

fn fetch_steamworks(
    remote_path: &str,
    library_name: &str,
    target_os: &str,
    target_arch: &str,
) -> Result<(), Box<dyn std::error::Error>> {
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

    let library = steamworks_library_path(&root, version, target_os, target_arch, library_name);
    fs::create_dir_all(library.parent().expect("library path has no parent"))?;

    download_if_missing(
        &library,
        &format!(
            "{STEAMWORKS_BASE_URL}/v{version}/steamworks-sys/lib/steam/redistributable_bin/{remote_path}"
        ),
    )?;

    update_link(&root.join(library_name), &library)?;

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
