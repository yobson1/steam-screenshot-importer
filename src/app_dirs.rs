use directories::ProjectDirs;
use std::sync::LazyLock;

pub static PROJECT_DIRS: LazyLock<ProjectDirs> =
    LazyLock::new(|| ProjectDirs::from("xyz", "yobson", "steam-screenshot-importer").unwrap());
